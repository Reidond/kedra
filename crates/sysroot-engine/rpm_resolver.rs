//! One-shot Fedora package resolution in disposable native containers.
//!
//! This is the only networked RPM step. It runs DNF once, as root inside a
//! container of the exact base image, never on the host. Every RPM the
//! transactions downloaded becomes a fixed-output store object, and the result
//! records what later offline installation must reproduce. The last builder role
//! installs in the resolved foundation container itself; any earlier role runs
//! in a container from one commit of it. Every role thus sees the same
//! repository metadata and the same untouched parent. (`dnf5 history undo` on a
//! bootc foundation aborts in libdnf5 5.4.6 on packages without a recorded
//! reason, so roles are not undone in place.) Containers and the commit carry
//! this store's token; a resolution of the same store removes any left by an
//! interrupted predecessor, because the store lock excludes a concurrent owner.
use crate::rpm::{
    MAX_RPM_BYTES, MAX_RPMS, RpmFile, RpmRequest, RpmResolution, RpmRole, koji_url, package_name,
    pinned_image, rpm_architectures, rpm_object, validate_rows,
};
use crate::{Error, Result, RunResult, Store, executor, store, tree};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const ROW_FORMAT: &str =
    "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\n";
const FILE_FORMAT: &str = "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\t%{SOURCERPM}\t%{RSAHEADER:pgpsig}\n";
const LOCATION_FORMAT: &str =
    "%{name}\t%{epoch}\t%{version}\t%{release}\t%{arch}\t%{repoid}\t%{location}\n";
const CACHE: &str = "/var/cache/libdnf5";
const COLLECT: &str = "/tmp/kedra-rpm-lock";
const LABEL: &str = "sysroot.engine.rpm-resolution";
const DNF: &str = "/usr/bin/dnf5";

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn oci_architecture(architecture: &str) -> Result<&'static str> {
    match architecture {
        "aarch64" => Ok("arm64"),
        "x86_64" => Ok("amd64"),
        _ => Err(invalid("RPM resolution supports only aarch64 and x86_64")),
    }
}

fn validate_request(request: &RpmRequest) -> Result<()> {
    rpm_architectures(&request.architecture)?;
    let names = |set: &BTreeSet<String>| set.len() <= 512 && set.iter().all(|n| package_name(n));
    let repositories = !request.repositories.is_empty()
        && request.repositories.len() <= 16
        && request.repositories.iter().all(|r| package_name(r));
    let roles = request.builders.len() <= 16
        && request.builders.iter().all(|(role, set)| {
            !role.is_empty() && role.len() <= 256 && !set.is_empty() && names(set)
        });
    if !pinned_image(&request.base)
        || !repositories
        || request.install.is_empty()
        || !names(&request.install)
        || !names(&request.remove)
        || !roles
    {
        return Err(invalid(
            "RPM resolution needs a digest-pinned base, repositories and safe package requests",
        ));
    }
    Ok(())
}

/// Rows of `rows` that `parent` does not already hold. A package identity
/// with other content than its parent's is refused.
fn delta(parent: &[Vec<String>], rows: &[Vec<String>]) -> Result<Vec<Vec<String>>> {
    let known: BTreeMap<&[String], &Vec<String>> =
        parent.iter().map(|row| (&row[..5], row)).collect();
    let mut added = Vec::new();
    for row in rows {
        match known.get(&row[..5]) {
            Some(previous) if *previous == row => {}
            Some(_) => {
                return Err(Error::Corrupt(format!(
                    "{} changed content without a version change",
                    row[0]
                )));
            }
            None => added.push(row.clone()),
        }
    }
    Ok(added)
}

fn file_name(row: &[String]) -> String {
    format!("{}-{}-{}.{}.rpm", row[0], row[2], row[3], row[4])
}

fn nevra(row: &[String]) -> String {
    format!("{}-{}:{}-{}.{}", row[0], row[1], row[2], row[3], row[4])
}

/// The Koji signed-copy directory: the low 32 bits of the OpenPGP key ID.
fn signing_key(signature: &str) -> Result<String> {
    let id = signature
        .rsplit_once("Key ID ")
        .map(|(_, id)| id.trim())
        .filter(|id| id.len() == 16 && id.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| {
            invalid(format!(
                "RPM is not signed with an OpenPGP key ID: {signature}"
            ))
        })?;
    Ok(id[8..].to_ascii_lowercase())
}

struct Header {
    row: Vec<String>,
    source_rpm: String,
    key: String,
}

fn parse_headers(output: &str, expected: usize) -> Result<Vec<Header>> {
    let headers = output
        .lines()
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            if fields.len() != 9 {
                return Err(invalid("unexpected RPM header query output"));
            }
            Ok(Header {
                row: fields[..7].iter().map(|s| (*s).to_owned()).collect(),
                source_rpm: fields[7].to_owned(),
                key: signing_key(fields[8])?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if headers.len() != expected {
        return Err(invalid("RPM header query returned another file count"));
    }
    Ok(headers)
}

/// Repository and mirror location per package identity; with the same identity
/// in several repositories the lowest repository name is recorded.
fn parse_locations(output: &str) -> Result<BTreeMap<Vec<String>, (String, String)>> {
    let mut locations: BTreeMap<Vec<String>, (String, String)> = BTreeMap::new();
    for line in output.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 7 {
            return Err(invalid("unexpected repository location output"));
        }
        let identity: Vec<String> = fields[..5].iter().map(|s| (*s).to_owned()).collect();
        let candidate = (fields[5].to_owned(), fields[6].to_owned());
        let keep = locations
            .get(&identity)
            .is_none_or(|current| candidate.0 < current.0);
        if keep {
            locations.insert(identity, candidate);
        }
    }
    Ok(locations)
}

fn parse_rows(output: &str, architecture: &str) -> Result<Vec<Vec<String>>> {
    let mut rows: Vec<Vec<String>> = output
        .lines()
        .filter(|line| !line.starts_with("gpg-pubkey\t"))
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect();
    rows.sort();
    validate_rows(&rows, architecture)?;
    Ok(rows)
}

struct Resolver<'a> {
    store: &'a Store,
    connection: executor::Connection,
    nonce: String,
    platform: String,
    architecture: String,
    repositories: Vec<String>,
    logs: Option<PathBuf>,
    sequence: usize,
    containers: Vec<String>,
    images: Vec<String>,
}

impl Store {
    /// Resolve the request once against Fedora repositories and admit every
    /// downloaded RPM as a fixed-output object. `logs` receives each command's
    /// bounded output when given.
    pub fn resolve_rpms(&self, request: &RpmRequest, logs: Option<&Path>) -> Result<RpmResolution> {
        validate_request(request)?;
        if let Some(path) = logs {
            fs::create_dir_all(path)?;
        }
        let mut resolver = Resolver {
            store: self,
            connection: self.connection()?,
            nonce: store::nonce()?[..16].to_owned(),
            platform: format!("linux/{}", oci_architecture(&request.architecture)?),
            architecture: request.architecture.clone(),
            repositories: request.repositories.clone(),
            logs: logs.map(Path::to_path_buf),
            sequence: 0,
            containers: Vec::new(),
            images: Vec::new(),
        };
        let stage = self.staging()?;
        let result = resolver.resolve(request, &stage);
        let cleanup = resolver.cleanup();
        let removed = tree::remove(&stage);
        let resolution = result?;
        cleanup?;
        removed?;
        Ok(resolution)
    }
}

impl Resolver<'_> {
    fn resolve(&mut self, request: &RpmRequest, stage: &Path) -> Result<RpmResolution> {
        self.check_daemon()?;
        self.remove_stale()?;
        self.ensure_image(&request.base)?;
        let foundation = self.start(&request.base)?;
        let base_rows = self.rows(&foundation)?;
        self.dnf(
            &foundation,
            "upgrade",
            &["--refresh", "upgrade"],
            &BTreeSet::new(),
        )?;
        self.dnf(&foundation, "install", &["install"], &request.install)?;
        if !request.remove.is_empty() {
            self.dnf(&foundation, "remove", &["remove"], &request.remove)?;
        }
        let rows = self.rows(&foundation)?;
        let mut rpms = BTreeMap::new();
        let install = self.collect(
            &foundation,
            &delta(&base_rows, &rows)?,
            &stage.join("0"),
            &mut rpms,
        )?;
        let mut builders = BTreeMap::new();
        let mut committed: Option<String> = None;
        for (index, (role, requests)) in request.builders.iter().enumerate() {
            let container = if index + 1 == request.builders.len() {
                foundation.clone()
            } else {
                let image = match &committed {
                    Some(image) => image.clone(),
                    None => committed.insert(self.commit(&foundation)?).clone(),
                };
                self.start(&image)?
            };
            let destination = stage.join((index + 1).to_string());
            let resolved = self.builder(&container, requests, &rows, &destination, &mut rpms)?;
            builders.insert(role.clone(), resolved);
        }
        if rpms.len() > MAX_RPMS {
            return Err(invalid("resolution exceeds the RPM count bound"));
        }
        Ok(RpmResolution {
            rpms,
            foundation: RpmRole {
                requests: request.install.clone(),
                remove: request.remove.clone(),
                install,
                packages: rows,
            },
            builders,
        })
    }

    /// Install one builder role over an untouched foundation and record it.
    fn builder(
        &mut self,
        container: &str,
        requests: &BTreeSet<String>,
        foundation: &[Vec<String>],
        destination: &Path,
        rpms: &mut BTreeMap<String, RpmFile>,
    ) -> Result<RpmRole> {
        self.dnf(container, "builder-install", &["install"], requests)?;
        let rows = self.rows(container)?;
        let added = delta(foundation, &rows)?;
        let install = self.collect(container, &added, destination, rpms)?;
        Ok(RpmRole {
            requests: requests.clone(),
            remove: BTreeSet::new(),
            install,
            packages: rows,
        })
    }

    /// Copy the RPM files of `added` out of the container cache and admit them.
    fn collect(
        &mut self,
        container: &str,
        added: &[Vec<String>],
        destination: &Path,
        rpms: &mut BTreeMap<String, RpmFile>,
    ) -> Result<BTreeSet<String>> {
        if added.is_empty() {
            return Ok(BTreeSet::new());
        }
        let cached = self.cached(container)?;
        let mut paths = Vec::new();
        for row in added {
            let file = file_name(row);
            let path = cached
                .get(&file)
                .ok_or_else(|| invalid(format!("DNF left no cached file for {file}")))?;
            paths.push(path.clone());
        }
        let headers = self.headers(container, &paths, added)?;
        let locations = self.locations(container, added)?;
        self.copy_out(container, &paths, destination)?;
        let mut files = BTreeSet::new();
        for (row, header) in added.iter().zip(headers) {
            let file = file_name(row);
            let local = destination.join(&file);
            let rpm = self.describe(row, header, &locations, &local)?;
            self.store.import_rpm(&local, &rpm)?;
            rpms.entry(file.clone()).or_insert(rpm);
            files.insert(file);
        }
        Ok(files)
    }

    fn describe(
        &self,
        row: &[String],
        header: Header,
        locations: &BTreeMap<Vec<String>, (String, String)>,
        local: &Path,
    ) -> Result<RpmFile> {
        let file = file_name(row);
        let (repository, mirror_url) = locations
            .get(&row[..5].to_vec())
            .ok_or_else(|| invalid(format!("no repository location for {file}")))?;
        let metadata = fs::symlink_metadata(local)?;
        if !metadata.is_file() || metadata.len() > MAX_RPM_BYTES {
            return Err(invalid(format!("{file}: copied RPM is not a bounded file")));
        }
        let (sha256, size) = store::hash_file(local, MAX_RPM_BYTES)?;
        let rpm = RpmFile {
            name: row[0].clone(),
            epoch: row[1].clone(),
            version: row[2].clone(),
            release: row[3].clone(),
            arch: row[4].clone(),
            size,
            object: rpm_object(&file, size, &sha256)?,
            sha256,
            header_sha256: row[5].clone(),
            payload_sha256: row[6].clone(),
            durable_url: koji_url(&header.source_rpm, &header.key, &row[4], &file)?,
            source_rpm: header.source_rpm,
            repository: repository.clone(),
            mirror_url: mirror_url.clone(),
        };
        rpm.validate(&file, &self.architecture)?;
        Ok(rpm)
    }

    fn cached(&mut self, container: &str) -> Result<BTreeMap<String, String>> {
        let listing = self.exec(
            container,
            "cache-list",
            &[
                "/usr/bin/find",
                CACHE,
                "-type",
                "f",
                "-path",
                "*/packages/*.rpm",
            ],
        )?;
        let mut cached = BTreeMap::new();
        for path in listing.lines() {
            if let Some((_, name)) = path.rsplit_once('/') {
                cached
                    .entry(name.to_owned())
                    .or_insert_with(|| path.to_owned());
            }
        }
        Ok(cached)
    }

    fn headers(
        &mut self,
        container: &str,
        paths: &[String],
        rows: &[Vec<String>],
    ) -> Result<Vec<Header>> {
        let mut argv = vec![
            "/usr/bin/rpm",
            "-qp",
            "--nosignature",
            "--nodigest",
            "--qf",
            FILE_FORMAT,
        ];
        argv.extend(paths.iter().map(String::as_str));
        let output = self.exec(container, "rpm-headers", &argv)?;
        let headers = parse_headers(&output, rows.len())?;
        for (row, header) in rows.iter().zip(&headers) {
            if header.row != *row {
                return Err(Error::Corrupt(format!(
                    "cached {} differs from the installed package",
                    file_name(row)
                )));
            }
        }
        Ok(headers)
    }

    fn locations(
        &mut self,
        container: &str,
        rows: &[Vec<String>],
    ) -> Result<BTreeMap<Vec<String>, (String, String)>> {
        let repositories: Vec<String> = self
            .repositories
            .iter()
            .map(|r| format!("--repo={r}"))
            .collect();
        let nevras: Vec<String> = rows.iter().map(|row| nevra(row)).collect();
        let mut argv = vec![DNF, "-q"];
        argv.extend(repositories.iter().map(String::as_str));
        argv.extend([
            "--setopt=*.metadata_expire=-1",
            "repoquery",
            "--queryformat",
            LOCATION_FORMAT,
        ]);
        argv.extend(nevras.iter().map(String::as_str));
        parse_locations(&self.exec(container, "repository-locations", &argv)?)
    }

    fn copy_out(&mut self, container: &str, paths: &[String], destination: &Path) -> Result<()> {
        // Each role collects into its own directory of the shared container.
        let collect = format!("{COLLECT}-{}", self.sequence);
        self.exec(container, "collect-dir", &["/usr/bin/mkdir", &collect])?;
        let mut argv = vec!["/usr/bin/ln", "--"];
        argv.extend(paths.iter().map(String::as_str));
        argv.push(&collect);
        self.exec(container, "collect-link", &argv)?;
        store::safe_bind(destination)?;
        fs::create_dir(destination)?;
        let source = format!("{container}:{collect}/.");
        let destination = destination
            .to_str()
            .ok_or_else(|| invalid("non-UTF8 staging path"))?;
        self.docker("copy-out", &["cp", &source, destination], 1800, false)?;
        Ok(())
    }

    fn rows(&mut self, container: &str) -> Result<Vec<Vec<String>>> {
        let output = self.exec(
            container,
            "rpm-inventory",
            &["/usr/bin/rpm", "-qa", "--qf", ROW_FORMAT],
        )?;
        parse_rows(&output, &self.architecture)
    }

    fn dnf(
        &mut self,
        container: &str,
        label: &str,
        verb: &[&str],
        packages: &BTreeSet<String>,
    ) -> Result<()> {
        let mut argv: Vec<String> = vec![
            "exec".into(),
            "--env".into(),
            "LC_ALL=C.UTF-8".into(),
            container.into(),
            DNF.into(),
            "-y".into(),
            "--best".into(),
        ];
        argv.extend(self.repositories.iter().map(|r| format!("--repo={r}")));
        argv.extend(
            [
                "--setopt=*.skip_if_unavailable=False",
                "--setopt=*.gpgcheck=True",
                "--setopt=keepcache=True",
            ]
            .map(String::from),
        );
        if verb[0] != "--refresh" {
            argv.push("--setopt=*.metadata_expire=-1".into());
        }
        argv.extend(verb.iter().map(|s| (*s).to_owned()));
        argv.extend(packages.iter().cloned());
        let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
        self.docker(label, &argv, 3600, false)?;
        Ok(())
    }

    fn exec(&mut self, container: &str, label: &str, command: &[&str]) -> Result<String> {
        let mut argv = vec!["exec", "--env", "LC_ALL=C.UTF-8", container];
        argv.extend(command);
        let result = self.docker(label, &argv, 600, true)?;
        String::from_utf8(result.stdout)
            .map_err(|_| invalid(format!("{label} output is not UTF-8")))
    }

    fn check_daemon(&mut self) -> Result<()> {
        let info = self.docker(
            "daemon",
            &["info", "--format", "{{.OSType}}/{{.Architecture}}"],
            120,
            true,
        )?;
        let observed = String::from_utf8_lossy(&info.stdout).trim().to_owned();
        let native = match observed.as_str() {
            "linux/aarch64" | "linux/arm64" => "aarch64",
            "linux/x86_64" | "linux/amd64" => "x86_64",
            _ => "",
        };
        if native != self.architecture {
            return Err(invalid(format!(
                "RPM resolution for {} needs a native Linux Docker engine, found {observed}",
                self.architecture
            )));
        }
        Ok(())
    }

    fn ensure_image(&mut self, image: &str) -> Result<()> {
        let format = [
            "image",
            "inspect",
            "--format",
            "{{.Os}}/{{.Architecture}}",
            image,
        ];
        let present = self.docker("base-inspect", &format, 120, false);
        if present.is_err() {
            if !image.contains('@') {
                return Err(invalid(format!("local base image {image} is not present")));
            }
            let platform = self.platform.clone();
            self.docker(
                "base-pull",
                &["pull", "--platform", &platform, image],
                1800,
                false,
            )?;
        }
        let observed = self.docker("base-platform", &format, 120, true)?;
        if String::from_utf8_lossy(&observed.stdout).trim() != self.platform {
            return Err(invalid(format!("base image is not {}", self.platform)));
        }
        Ok(())
    }

    fn labels(&self) -> [String; 2] {
        [
            format!("sysroot.engine.store={}", self.store.token),
            format!("{LABEL}={}", self.nonce),
        ]
    }

    fn start(&mut self, image: &str) -> Result<String> {
        let name = format!(
            "sysroot-rpm-{}-{}-{}",
            &self.store.token[..12],
            self.nonce,
            self.containers.len()
        );
        let [store_label, operation] = self.labels();
        let platform = self.platform.clone();
        self.containers.push(name.clone());
        self.docker(
            "create",
            &[
                "create",
                "--pull",
                "never",
                "--name",
                &name,
                "--label",
                &store_label,
                "--label",
                &operation,
                "--platform",
                &platform,
                "--security-opt",
                "no-new-privileges",
                "--pids-limit",
                "4096",
                "--entrypoint",
                "/usr/bin/sleep",
                image,
                "infinity",
            ],
            300,
            false,
        )?;
        self.docker("start", &["start", &name], 300, false)?;
        Ok(name)
    }

    fn commit(&mut self, container: &str) -> Result<String> {
        let [store_label, operation] = self.labels();
        let result = self.docker(
            "commit",
            &[
                "commit",
                "--change",
                &format!("LABEL {store_label}"),
                "--change",
                &format!("LABEL {operation}"),
                container,
            ],
            3600,
            true,
        )?;
        let image = String::from_utf8_lossy(&result.stdout).trim().to_owned();
        crate::plan::image_id(&image)?;
        self.images.push(image.clone());
        Ok(image)
    }

    /// Owned leftovers of an interrupted resolution of this locked store.
    fn remove_stale(&mut self) -> Result<()> {
        let store_label = format!("label=sysroot.engine.store={}", self.store.token);
        let operation = format!("label={LABEL}");
        let filters = ["--filter", &store_label, "--filter", &operation];
        let containers = self.docker(
            "stale-containers",
            &[
                &["container", "ls", "--all", "--quiet", "--no-trunc"][..],
                &filters,
            ]
            .concat(),
            120,
            true,
        )?;
        for id in String::from_utf8_lossy(&containers.stdout).split_whitespace() {
            self.docker(
                "stale-remove",
                &["container", "rm", "--force", id],
                300,
                false,
            )?;
        }
        let images = self.docker(
            "stale-images",
            &[&["image", "ls", "--quiet", "--no-trunc"][..], &filters].concat(),
            120,
            true,
        )?;
        for id in String::from_utf8_lossy(&images.stdout).split_whitespace() {
            self.docker(
                "stale-image-remove",
                &["image", "rm", "--force", id],
                300,
                false,
            )?;
        }
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        for container in std::mem::take(&mut self.containers) {
            if let Err(error) = self.docker(
                "cleanup",
                &["container", "rm", "--force", &container],
                300,
                false,
            ) {
                failures.push(error.to_string());
            }
        }
        for image in std::mem::take(&mut self.images) {
            if let Err(error) = self.docker(
                "cleanup-image",
                &["image", "rm", "--force", &image],
                300,
                false,
            ) {
                failures.push(error.to_string());
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(Error::RecoveryRequired(format!(
                "resolver cleanup failed (labels sysroot.engine.store={}, {LABEL}={}): {}",
                self.store.token,
                self.nonce,
                failures.join("; ")
            )))
        }
    }

    /// One bounded Docker CLI call. `capture` requires complete stdout.
    fn docker(
        &mut self,
        label: &str,
        args: &[&str],
        seconds: u64,
        capture: bool,
    ) -> Result<RunResult> {
        let mut command = self.connection.command();
        command.args(args);
        let result = executor::command(command, seconds)?;
        self.sequence += 1;
        if let Some(logs) = &self.logs {
            let prefix = logs.join(format!("{:03}-{label}", self.sequence));
            fs::write(prefix.with_extension("stdout"), &result.stdout)?;
            fs::write(prefix.with_extension("stderr"), &result.stderr)?;
        }
        if result.code != 0 {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let tail: String = stderr
                .chars()
                .rev()
                .take(2000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            return Err(Error::Process {
                code: result.code,
                message: format!("{label}: {}", tail.trim()),
            });
        }
        if capture && result.stdout_truncated {
            return Err(invalid(format!("{label} output exceeds 1 MiB")));
        }
        Ok(result)
    }
}

/// Copy one bounded regular file out of a local image without running it.
/// `scratch` must be a new, absent path; it is removed afterwards.
pub fn image_file(image: &str, path: &str, limit: u64, scratch: &Path) -> Result<Vec<u8>> {
    if image.is_empty() || image.starts_with('-') || image.bytes().any(|b| b < 33 || b == 127) {
        return Err(invalid("invalid image reference"));
    }
    if !path.starts_with('/') || path.split('/').any(|part| part == "..") {
        return Err(invalid("image file path must be absolute"));
    }
    let connection = executor::Connection::resolve()?;
    let run = |args: &[&str]| -> Result<RunResult> {
        let mut command = connection.command();
        command.args(args);
        executor::checked(executor::command(command, 300)?)
    };
    store::safe_bind(scratch)?;
    fs::create_dir(scratch)?;
    let name = format!("sysroot-extract-{}", &store::nonce()?[..16]);
    let result = (|| {
        run(&[
            "create",
            "--pull",
            "never",
            "--network",
            "none",
            "--name",
            &name,
            "--entrypoint",
            "/usr/bin/false",
            image,
        ])?;
        let target = scratch.join("file");
        let destination = target
            .to_str()
            .ok_or_else(|| invalid("non-UTF8 scratch path"))?;
        run(&["cp", &format!("{name}:{path}"), destination])?;
        let metadata = fs::symlink_metadata(&target)?;
        if !metadata.is_file() || metadata.len() > limit {
            return Err(invalid(format!(
                "{path} in {image} is not a bounded regular file"
            )));
        }
        Ok(fs::read(&target)?)
    })();
    let removed = run(&["container", "rm", "--force", &name]);
    let cleaned = fs::remove_dir_all(scratch);
    let bytes = result?;
    removed?;
    cleaned?;
    Ok(bytes)
}
