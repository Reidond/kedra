use crate::{
    executor,
    plan::{hash, name, relative},
    store::{atomic_json, private, read_json, validate_run_args, write_json_new},
    *,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
};
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index {
    schema: u32,
    selected: u64,
    generations: BTreeMap<u64, String>,
}
impl Store {
    pub fn profile_switch(
        &self,
        profile: &str,
        id: &str,
        program: &str,
        args: &[String],
    ) -> Result<Profile> {
        name(profile)?;
        relative(program, false)?;
        validate_run_args(args)?;
        let receipt = self.verify(id)?;
        let image = receipt
            .runtime_image
            .ok_or_else(|| Error::Invalid("profile requires a realized output".into()))?;
        let path = self.path().join("profiles").join(profile);
        let is_new = !path.exists();
        let mut index = if !is_new {
            self.profile_list(profile)?;
            read_json::<Index>(&path.join("index.json"))?
        } else {
            Index {
                schema: 1,
                selected: 0,
                generations: BTreeMap::new(),
            }
        };
        let generation = index
            .generations
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| Error::Invalid("profile generation overflow".into()))?;
        let record = Generation {
            generation,
            object: id.into(),
            program: program.into(),
            args: args.into(),
            runtime_image: image,
        };
        let digest = hash(&serde_json::to_vec(&record)?);
        let stage = if is_new { Some(self.staging()?) } else { None };
        let working = stage.as_deref().unwrap_or(&path);
        let publication = (|| {
            let record_path = working.join(format!("{digest}.json"));
            if !record_path.exists() {
                write_json_new(&record_path, &record)?;
            } else {
                private(&record_path, false)?;
                let existing: Generation = read_json(&record_path)?;
                if hash(&serde_json::to_vec(&existing)?) != digest {
                    return Err(Error::Corrupt(
                        "unselected profile generation differs".into(),
                    ));
                }
            }
            index.generations.insert(generation, digest);
            index.selected = generation;
            atomic_json(&working.join("index.json"), &index)?;
            File::open(working)?.sync_all()?;
            if is_new {
                fs::rename(working, &path)?;
            }
            File::open(self.path().join("profiles"))?.sync_all()?;
            if is_new {
                File::open(self.path().join("transactions"))?.sync_all()?;
            }
            Ok(())
        })();
        if let Some(stage) = stage
            && stage.exists()
        {
            crate::tree::remove(&stage)?;
        }
        publication?;
        self.profile_list(profile)
    }
    pub fn profile_list(&self, profile: &str) -> Result<Profile> {
        name(profile)?;
        let path = self.path().join("profiles").join(profile);
        private(&path, true)?;
        private(&path.join("index.json"), false)?;
        let index: Index = read_json(&path.join("index.json"))?;
        if index.schema != 1
            || !index.generations.contains_key(&index.selected)
            || index.generations.len() > 100000
        {
            return Err(Error::Corrupt("invalid profile index".into()));
        }
        let mut generations = Vec::new();
        let mut object = String::new();
        for (number, digest) in index.generations {
            if !crate::plan::hex(&digest) {
                return Err(Error::Corrupt("invalid generation digest".into()));
            }
            let file = path.join(format!("{digest}.json"));
            private(&file, false)?;
            let generation: Generation = read_json(&file)?;
            if generation.generation != number || hash(&serde_json::to_vec(&generation)?) != digest
            {
                return Err(Error::Corrupt("profile generation content differs".into()));
            }
            relative(&generation.program, false)?;
            validate_run_args(&generation.args)?;
            let receipt = self.verify(&generation.object)?;
            if receipt.runtime_image.as_ref() != Some(&generation.runtime_image) {
                return Err(Error::Corrupt("profile runtime image differs".into()));
            }
            if number == index.selected {
                object = generation.object.clone();
            }
            generations.push(generation);
        }
        Ok(Profile {
            name: profile.into(),
            selected: index.selected,
            object,
            generations,
        })
    }
    pub fn profile_rollback(&self, profile: &str) -> Result<Profile> {
        self.profile_list(profile)?;
        let path = self
            .path()
            .join("profiles")
            .join(profile)
            .join("index.json");
        let mut index: Index = read_json(&path)?;
        index.selected = index
            .generations
            .range(..index.selected)
            .next_back()
            .map(|(number, _)| *number)
            .ok_or_else(|| Error::Invalid("profile has no previous generation".into()))?;
        atomic_json(&path, &index)?;
        self.profile_list(profile)
    }
    pub fn profile_run(&self, profile: &str, args: &[String], seconds: u64) -> Result<RunResult> {
        let profile = self.profile_list(profile)?;
        let generation = profile
            .generations
            .iter()
            .find(|g| g.generation == profile.selected)
            .ok_or_else(|| Error::Corrupt("selected generation missing".into()))?;
        let mut argv = generation.args.clone();
        argv.extend_from_slice(args);
        self.run_output(&generation.object, &generation.program, &argv, seconds)
    }
    pub fn develop(
        &self,
        profile: &str,
        program: Option<&str>,
        args: &[String],
        seconds: u64,
    ) -> Result<RunResult> {
        let Some(program) = program else {
            return self.profile_run(profile, args, seconds);
        };
        let relative_program = program
            .strip_prefix('/')
            .ok_or_else(|| Error::Invalid("develop image program must be absolute".into()))?;
        relative(relative_program, false)?;
        if program.starts_with(LOGICAL_PREFIX) {
            return Err(Error::Invalid(
                "use profile executable for store programs".into(),
            ));
        }
        let profile = self.profile_list(profile)?;
        let generation = profile
            .generations
            .iter()
            .find(|g| g.generation == profile.selected)
            .ok_or_else(|| Error::Corrupt("selected generation missing".into()))?;
        let closure = self.closure(&generation.object)?;
        let mut argv = vec![program.into()];
        argv.extend_from_slice(args);
        validate_run_args(&argv)?;
        executor::execute(
            self,
            executor::Execution {
                image: &generation.runtime_image,
                mounts: &closure.objects,
                output: None,
                argv: &argv,
                env: &BTreeMap::new(),
                seconds,
            },
        )
    }
}
