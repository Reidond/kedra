//! One running Kedra OS container: systemd as PID 1 on a lab image.
//!
//! Each scenario gets its own container (dedicated isolation: every mutable
//! service it touches starts fresh). The lab image is the shared, read-only
//! part of an execution.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use testcontainers::core::{CgroupnsMode, Host, Mount};
use testcontainers::runners::SyncRunner;
use testcontainers::{Container, GenericImage, ImageExt};

use crate::docker::{self, Docker, Exec, Output};
use crate::image::LabImage;
use crate::{Error, Result};

/// What a container is for; recorded as an ownership label.
#[derive(Clone, Debug)]
pub enum Kind {
    /// A scenario or native test in one execution.
    Test { execution: String, test: String },
    /// A retained interactive lab environment.
    Lab { name: String },
}

/// Extra host resources, e.g. the waypipe socket directory for live viewing.
#[derive(Clone, Debug, Default)]
pub struct Extras {
    pub bind_mounts: Vec<(String, String)>,
}

pub struct Environment {
    container: Option<Container<GenericImage>>,
    pub id: String,
    pub name: String,
    pub image: String,
    pub started: Instant,
}

fn sanitize(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    cleaned.trim_matches('-').chars().take(40).collect()
}

impl Environment {
    /// Start systemd on `image` and wait until boot finished.
    pub fn start(docker: &Docker, image: &LabImage, kind: &Kind, extras: &Extras) -> Result<Self> {
        let (name, kind_label, execution) = match kind {
            Kind::Test { execution, test } => (
                format!("kedra-test-{execution}-{}", sanitize(test)),
                "test",
                execution.clone(),
            ),
            Kind::Lab { name } => (format!("kedra-lab-{}", sanitize(name)), "lab", name.clone()),
        };
        let mut request = GenericImage::new(&image.name, &image.tag)
            .with_privileged(true)
            .with_cgroupns_mode(CgroupnsMode::Private)
            .with_mount(Mount::tmpfs_mount("/run"))
            .with_mount(Mount::tmpfs_mount("/run/lock"))
            .with_mount(Mount::tmpfs_mount("/tmp"))
            .with_env_var("container", "docker")
            .with_hostname("kedra-lab")
            // Engines on Linux need this mapping; macOS engines provide it.
            .with_host("host.docker.internal", Host::HostGateway)
            .with_container_name(&name)
            .with_label(docker::OWNER_LABEL, docker::OWNER)
            .with_label(docker::KIND_LABEL, kind_label)
            .with_label(docker::EXECUTION_LABEL, &execution)
            .with_label(docker::TARGET_LABEL, &image.target)
            .with_label(docker::IMAGE_LABEL, image.reference())
            .with_startup_timeout(Duration::from_secs(120));
        for (source, target) in &extras.bind_mounts {
            request = request.with_mount(Mount::bind_mount(source, target));
        }
        let container = request.start()?;
        let environment = Environment {
            id: container.id().to_owned(),
            container: Some(container),
            name,
            image: image.reference(),
            started: Instant::now(),
        };
        environment.wait_booted(docker, Duration::from_secs(90))?;
        Ok(environment)
    }

    /// Attach to a retained lab container by name or ID prefix.
    pub fn attach(docker: &Docker, selector: Option<&str>) -> Result<Self> {
        let owned = docker.owned(&[(docker::KIND_LABEL, "lab")])?;
        let matching: Vec<_> = owned
            .into_iter()
            .filter(|item| item.state == "running")
            .filter(|item| match selector {
                None => true,
                Some(selector) => {
                    item.name == selector
                        || item.name == format!("kedra-lab-{}", sanitize(selector))
                        || item.id.starts_with(selector)
                }
            })
            .collect();
        match matching.as_slice() {
            [one] => Ok(Environment {
                container: None,
                id: one.id.clone(),
                name: one.name.clone(),
                image: one
                    .labels
                    .get(docker::IMAGE_LABEL)
                    .cloned()
                    .unwrap_or_default(),
                started: Instant::now(),
            }),
            [] => Err(Error::Invalid(
                "no running lab environment; start one with `kedra-lab up`".into(),
            )),
            many => Err(Error::Invalid(format!(
                "{} lab environments are running; name one of: {}",
                many.len(),
                many.iter()
                    .map(|item| item.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }

    /// Wait for PID 1 to finish booting. `systemctl` answers `offline` until
    /// the manager accepts connections, so poll within one deadline.
    fn wait_booted(&self, docker: &Docker, limit: Duration) -> Result<()> {
        let deadline = Instant::now() + limit;
        let mut last = String::new();
        while Instant::now() < deadline {
            let remaining = deadline
                .saturating_duration_since(Instant::now())
                .max(Duration::from_secs(1));
            let exec = Exec::new(["systemctl", "is-system-running", "--wait"]).timeout(remaining);
            let output = docker.exec(&self.id, &exec)?;
            last = output.stdout_text().trim().to_owned();
            match last.as_str() {
                "running" | "degraded" => return Ok(()),
                "maintenance" | "stopping" => break,
                _ => std::thread::sleep(Duration::from_millis(200)),
            }
        }
        Err(Error::Timeout {
            what: format!("systemd boot in the container (last state {last:?})"),
            after: limit,
        })
    }

    pub fn exec(&self, docker: &Docker, exec: &Exec) -> Result<Output> {
        docker.exec(&self.id, exec)
    }

    pub fn run(&self, docker: &Docker, exec: &Exec) -> Result<Output> {
        docker.run(&self.id, exec)
    }

    /// Save journal, unit and process state for a failed or inspected run.
    /// Each item is best effort; the first error is returned after trying all.
    pub fn collect(&self, docker: &Docker, directory: &Path) -> Result<()> {
        fs::create_dir_all(directory)?;
        let items: [(&str, &[&str]); 5] = [
            (
                "journal.log",
                &["journalctl", "-b", "--no-pager", "-o", "short-iso-precise"],
            ),
            (
                "failed-units.txt",
                &["systemctl", "--failed", "--all", "--no-pager"],
            ),
            (
                "user-units.txt",
                &["systemctl", "list-units", "--all", "--no-pager", "user@*"],
            ),
            ("sessions.txt", &["loginctl", "list-sessions", "--no-pager"]),
            (
                "processes.txt",
                &["ps", "-eo", "pid,user,stat,etime,args", "--forest"],
            ),
        ];
        let mut first_error = None;
        for (file, argv) in items {
            let exec = Exec::new(argv.iter().copied()).timeout(Duration::from_secs(30));
            match docker.exec(&self.id, &exec) {
                Ok(output) => {
                    // Cap journal size: keep the tail, where failures are.
                    let mut data = output.stdout;
                    const LIMIT: usize = 8 * 1024 * 1024;
                    if data.len() > LIMIT {
                        data = data.split_off(data.len() - LIMIT);
                    }
                    fs::write(directory.join(file), data)?;
                }
                Err(error) => {
                    fs::write(directory.join(file), format!("not collected: {error}\n"))?;
                    first_error.get_or_insert(error);
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    /// Keep the container running after this process exits.
    pub fn retain(mut self) -> String {
        if let Some(container) = self.container.take() {
            // Dropping would remove it; the ownership labels allow later cleanup.
            std::mem::forget(container);
        }
        self.name.clone()
    }

    /// Stop and remove the container, reporting failures instead of hiding them.
    pub fn terminate(mut self, docker: &Docker) -> Result<()> {
        match self.container.take() {
            Some(container) => container.rm().map_err(Error::from),
            None => docker.remove(&self.id),
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        // Testcontainers removes an owned container when its handle drops,
        // which also covers panics between start and terminate.
        if self.container.is_some() {
            eprintln!("kedra-lab: removing {} after an interrupted run", self.name);
        }
    }
}
