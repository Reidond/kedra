//! A small application's definitions, independent of Kedra release authority.
use std::collections::{BTreeMap, BTreeSet};

use sysroot_catalog::{Catalog, Package, Policy, Recipe, Source, SourceOrigin};
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input};

pub fn definitions(source: &str, builder: &str, runtime: &str, version: &str) -> (Catalog, Policy) {
    let support = BuildNode {
        builder_image: builder.into(),
        runtime_image: runtime.into(),
        inputs: BTreeMap::from([("source".into(), Input::Object(source.into()))]),
        argv: vec![
            Argument::literal("/bin/sh"),
            Argument::input("source", "support.sh"),
            Argument::input("source", ""),
            Argument::output(""),
        ],
        env: BTreeMap::new(),
        runtime_inputs: vec![],
        timeout_seconds: 120,
    };
    let report = BuildNode {
        builder_image: builder.into(),
        runtime_image: runtime.into(),
        inputs: BTreeMap::from([
            ("source".into(), Input::Object(source.into())),
            ("support".into(), Input::Node("support".into())),
        ]),
        argv: vec![
            Argument::literal("/bin/sh"),
            Argument::input("source", "report.sh"),
            Argument::input("source", "report.c"),
            Argument::input("support", ""),
            Argument::output(""),
            Argument::literal(crate::PROGRAM),
        ],
        env: BTreeMap::new(),
        runtime_inputs: vec!["support".into()],
        timeout_seconds: 120,
    };
    let package = Package {
        version: version.into(),
        summary: format!("{} sensor report", crate::PROJECT),
        license: "MIT".into(),
        sources: vec![Source {
            object: source.into(),
            origin: SourceOrigin::Local {
                description: format!("Explicitly selected {} source", crate::PROJECT),
            },
        }],
        recipe: Recipe {
            graph: BuildGraph {
                schema: 1,
                nodes: BTreeMap::from([("support".into(), support), ("report".into(), report)]),
            },
            root: "report".into(),
            program: crate::PROGRAM.into(),
        },
    };
    (
        Catalog {
            namespace: crate::PROJECT.into(),
            packages: BTreeMap::from([("report".into(), package)]),
        },
        Policy {
            namespace: crate::PROJECT.into(),
            packages: BTreeSet::from(["report".into()]),
            builder_images: BTreeSet::from([builder.into()]),
            runtime_images: BTreeSet::from([runtime.into()]),
            source_objects: BTreeSet::from([source.into()]),
        },
    )
}
