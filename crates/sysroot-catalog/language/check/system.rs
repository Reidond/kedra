//! System declarations of sets and targets: image files, the home baseline and
//! its managed files, unit operations, target metadata and foundation edits.
use super::{Checker, Content, Definition, Kind, Result, Type, Value, fail};
use crate::language::{Block, MAX_RESOURCES, public_input, relative};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use sysroot_engine::NativeStep;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileIntent {
    /// The declaring set or target, `<module>#<declaration>`.
    pub origin: String,
    pub content: Content,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HomeIntent {
    pub origin: String,
    pub content: Content,
    /// Managed by `sysroot home`: merged on update, checked, and captured back
    /// to its source file.
    pub managed: bool,
    pub check: Option<Vec<String>>,
    pub reload: Option<Vec<String>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OsReleaseIntent {
    pub name: String,
    pub pretty_name: String,
}

/// Unit operations the selected sets contribute to the target's systemd step.
#[derive(Default)]
pub(super) struct Selection {
    enable: BTreeSet<String>,
    disable: BTreeSet<String>,
    mask: BTreeSet<String>,
}

/// Image files live under /etc or /usr. The home baseline, bootc's /usr/etc,
/// the development tree and the engine's generated manifests are not files.
fn system_path(path: &str) -> bool {
    let Some(rest) = path.strip_prefix('/') else {
        return false;
    };
    let reserved = ["etc/skel", "usr/etc", "usr/src", "usr/share/sysroot"];
    relative(rest)
        && public_input(rest, &[])
        && rest.contains('/')
        && matches!(rest.split('/').next(), Some("etc" | "usr"))
        && !reserved
            .iter()
            .any(|prefix| rest == *prefix || rest.starts_with(&format!("{prefix}/")))
}
fn home_path(path: &str) -> bool {
    relative(path) && public_input(path, &[]) && !path.ends_with(".kedra-new")
}
/// XDG desktop entry and desktop names, as `NotShowIn=` lists them.
fn desktop_entry(name: &str) -> bool {
    name.strip_suffix(".desktop").is_some_and(|stem| {
        !stem.is_empty()
            && stem
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    }) && name.len() <= 255
}
fn desktop_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
/// os-release values stay printable without quoting or expansion characters.
fn release_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| (b' '..=b'~').contains(&b) && !b"\"\\$`".contains(&b))
}
fn image_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"./_-".contains(&b))
}
fn hardware_status(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|b| b.is_ascii_lowercase() || b == b'-')
}

impl Checker {
    /// Field checks of a set or target, applied to every declaration.
    pub(super) fn validate_system_field(
        &self,
        definition: &Definition,
        name: &str,
        value: &Value,
    ) -> Result<()> {
        let module = &definition.module;
        let text = |check: fn(&str) -> bool, message| -> Result<()> {
            if check(&self.string(module, value)?) {
                Ok(())
            } else {
                Err(self.error(module, value, name, message))
            }
        };
        match name {
            "files" => self.validate_files(definition, value),
            "home" => self.validate_home(definition, value),
            "hide_autostart" => self.validate_autostart(module, value),
            "systemd" if definition.declaration.kind == "set" => self
                .construct(module, value, "systemd", &["enable", "disable", "mask"])
                .map(drop),
            "image" => text(image_name, "invalid image repository"),
            "hardware" => text(hardware_status, "invalid hardware status"),
            _ => Ok(()),
        }
    }
    fn validate_files(&self, definition: &Definition, value: &Value) -> Result<()> {
        let module = &definition.module;
        let files = self.map(module, value)?;
        if files.len() > MAX_RESOURCES {
            return Err(self.error(module, value, "limit", "resource count exceeded"));
        }
        for (path, content) in files {
            if !system_path(path) {
                return Err(self.error(
                    module,
                    content,
                    "path",
                    "image file needs an allowed absolute /etc or /usr path",
                ));
            }
            if self.typed(definition, content)? != Type::Content {
                return Err(self.error(module, content, "type", "file requires content"));
            }
        }
        Ok(())
    }
    fn validate_home(&self, definition: &Definition, value: &Value) -> Result<()> {
        let module = &definition.module;
        let files = self.map(module, value)?;
        if files.len() > MAX_RESOURCES {
            return Err(self.error(module, value, "limit", "resource count exceeded"));
        }
        let mut names = BTreeSet::new();
        for (path, item) in files {
            if !home_path(path) || !names.insert(path.to_ascii_lowercase()) {
                return Err(self.error(
                    module,
                    item,
                    "path",
                    "unsafe, private or case-colliding home path",
                ));
            }
            if !matches!(self.typed(definition, item)?, Type::Content | Type::Managed) {
                return Err(self.error(
                    module,
                    item,
                    "type",
                    "home file requires content or managed",
                ));
            }
        }
        Ok(())
    }
    fn validate_autostart(&self, module: &str, value: &Value) -> Result<()> {
        for (entry, desktops) in self.map(module, value)? {
            let names = self.list(module, desktops)?;
            if !desktop_entry(entry) || names.is_empty() {
                return Err(self.error(module, desktops, "autostart", "invalid desktop entry"));
            }
            for name in names {
                if !desktop_name(&self.string(module, name)?) {
                    return Err(self.error(module, name, "autostart", "invalid desktop name"));
                }
            }
        }
        Ok(())
    }
    pub(super) fn typed_managed(
        &self,
        definition: &Definition,
        value: &Value,
        block: &Block,
    ) -> Result<Type> {
        let module = &definition.module;
        self.construct(module, value, "managed", &["content", "check", "reload"])?;
        let content = block
            .fields
            .get("content")
            .ok_or_else(|| self.error(module, value, "field", "managed requires content"))?;
        // Capture writes the live file back to this source file.
        if self.typed(definition, content)? != Type::Content
            || !matches!(self.content(definition, content)?, Content::File { .. })
        {
            return Err(self.error(
                module,
                content,
                "managed",
                "managed home content must be an admitted file(\"./…\")",
            ));
        }
        for field in ["check", "reload"] {
            if let Some(argv) = block.fields.get(field) {
                self.command(module, argv)?;
            }
        }
        Ok(Type::Managed)
    }
    /// An argv list with an absolute program, never shell text.
    fn command(&self, module: &str, value: &Value) -> Result<Vec<String>> {
        let argv = self
            .list(module, value)?
            .iter()
            .map(|item| self.string(module, item))
            .collect::<Result<Vec<_>>>()?;
        if argv.is_empty()
            || argv.len() > 64
            || !argv[0].starts_with('/')
            || !relative(&argv[0][1..])
            || argv
                .iter()
                .any(|argument| argument.len() > 4096 || argument.chars().any(char::is_control))
        {
            return Err(self.error(
                module,
                value,
                "command",
                "expected argv with an absolute program",
            ));
        }
        Ok(argv)
    }
    pub(super) fn typed_os_release(
        &self,
        definition: &Definition,
        value: &Value,
        block: &Block,
    ) -> Result<Type> {
        let module = &definition.module;
        self.construct(module, value, "os_release", &["name", "pretty_name"])?;
        for field in ["name", "pretty_name"] {
            let item = block
                .fields
                .get(field)
                .ok_or_else(|| self.error(module, value, "field", "os_release needs both names"))?;
            if !release_value(&self.string(module, item)?) {
                return Err(self.error(module, item, "os_release", "invalid os-release value"));
            }
        }
        Ok(Type::OsRelease)
    }

    /// Image metadata and foundation edits only a target declares.
    pub(super) fn target_metadata(&mut self, definition: &Definition) -> Result<()> {
        let module = &definition.module;
        let fields = &definition.declaration.block.fields;
        let optional = |name: &str| -> Result<Option<String>> {
            fields
                .get(name)
                .map(|value| self.string(module, value))
                .transpose()
        };
        let image = optional("image")?;
        let hardware = optional("hardware")?;
        self.intent.image = image;
        self.intent.hardware = hardware;
        if let Some(value) = fields.get("os_release") {
            let block = self.construct(module, value, "os_release", &["name", "pretty_name"])?;
            self.intent.os_release = Some(OsReleaseIntent {
                name: self.string(module, &block.fields["name"])?,
                pretty_name: self.string(module, &block.fields["pretty_name"])?,
            });
        }
        if let Some(value) = fields.get("hide_autostart") {
            for (entry, desktops) in self.map(module, value)? {
                let names = self
                    .list(module, desktops)?
                    .iter()
                    .map(|name| self.string(module, name))
                    .collect::<Result<BTreeSet<_>>>()?;
                self.intent.hide_autostart.insert(entry.clone(), names);
            }
        }
        Ok(())
    }

    /// Files, home files and unit operations of one selected set or target.
    /// A path declared twice is refused with both origins: modules compose by
    /// `use`, never by overriding each other.
    pub(super) fn collect_system(&mut self, key: &str, definition: &Definition) -> Result<()> {
        let module = &definition.module;
        let fields = &definition.declaration.block.fields;
        if let Some(value) = fields.get("files") {
            for (path, item) in self.map(module, value)? {
                let file = FileIntent {
                    origin: key.into(),
                    content: self.content(definition, item)?,
                };
                if let Some(previous) = self.intent.files.insert(path.clone(), file) {
                    return Err(self.conflict(
                        &previous.origin,
                        key,
                        "file",
                        "file declared twice",
                    ));
                }
            }
        }
        if let Some(value) = fields.get("home") {
            for (path, item) in self.map(module, value)? {
                let home = self.home_file(definition, key, item)?;
                if let Some(previous) = self.intent.home.insert(path.clone(), home) {
                    return Err(self.conflict(
                        &previous.origin,
                        key,
                        "home",
                        "home file declared twice",
                    ));
                }
            }
        }
        if definition.declaration.kind == "set"
            && let Some(value) = fields.get("systemd")
        {
            let block = self.construct(module, value, "systemd", &["enable", "disable", "mask"])?;
            let enable = self.step_strings(module, block, "enable")?;
            let disable = self.step_strings(module, block, "disable")?;
            let mask = self.step_strings(module, block, "mask")?;
            self.system.enable.extend(enable);
            self.system.disable.extend(disable);
            self.system.mask.extend(mask);
        }
        Ok(())
    }
    fn home_file(&self, definition: &Definition, key: &str, item: &Value) -> Result<HomeIntent> {
        let module = &definition.module;
        let Kind::Construct(name, block) = &item.kind else {
            return Ok(HomeIntent {
                origin: key.into(),
                content: self.content(definition, item)?,
                managed: false,
                check: None,
                reload: None,
            });
        };
        if name != "managed" {
            return Err(self.error(module, item, "type", "expected managed"));
        }
        let command = |field: &str| {
            block
                .fields
                .get(field)
                .map(|argv| self.command(module, argv))
                .transpose()
        };
        Ok(HomeIntent {
            origin: key.into(),
            content: self.content(definition, &block.fields["content"])?,
            managed: true,
            check: command("check")?,
            reload: command("reload")?,
        })
    }

    /// Merge the sets' unit operations into the target's systemd step, add the
    /// managed files' checks, and order the steps as the engine requires.
    pub(super) fn finish_system(
        &mut self,
        definition: &Definition,
        steps: &mut Vec<NativeStep>,
    ) -> Result<()> {
        let units = std::mem::take(&mut self.system);
        if !(units.enable.is_empty() && units.disable.is_empty() && units.mask.is_empty()) {
            let position = steps
                .iter()
                .position(|step| matches!(step, NativeStep::Systemd { .. }));
            let position = position.unwrap_or_else(|| {
                steps.push(NativeStep::Systemd {
                    enable: Vec::new(),
                    disable: Vec::new(),
                    mask: Vec::new(),
                    default_target: None,
                });
                steps.len() - 1
            });
            if let NativeStep::Systemd {
                enable,
                disable,
                mask,
                ..
            } = &mut steps[position]
            {
                for (list, extra) in [
                    (enable, units.enable),
                    (disable, units.disable),
                    (mask, units.mask),
                ] {
                    let merged: BTreeSet<String> = list.drain(..).chain(extra).collect();
                    list.extend(merged);
                }
            }
        }
        let checks: BTreeMap<String, Vec<String>> = self
            .intent
            .home
            .iter()
            .filter_map(|(path, home)| home.check.clone().map(|argv| (path.clone(), argv)))
            .collect();
        if !checks.is_empty() {
            steps.push(NativeStep::HomeChecks { checks });
        }
        if !steps.is_empty() {
            sysroot_engine::normalize_native_steps(steps).map_err(|error| {
                fail(
                    &definition.module,
                    definition.declaration.span,
                    "image",
                    &error.to_string(),
                )
            })?;
        }
        Ok(())
    }
}
