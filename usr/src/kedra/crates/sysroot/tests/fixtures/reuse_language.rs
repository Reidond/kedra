//! An independent, separately compiled consumer authors typed intent directly.
use std::collections::{BTreeMap, BTreeSet};
use sysroot_catalog::{Policy, language::{self, Content, ExportIntent, Images, Intent, RecipeIntent, SourceIntent}};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let builder = args.get(1).expect("exact builder").clone();
    let runtime = args.get(2).expect("exact runtime").clone();
    let key = "catalog.kedra#hello".to_owned();
    let recipe = RecipeIntent {
        module:"catalog.kedra".into(), name:"hello".into(), library:false,
        version:"1".into(), summary:"Inline program".into(), license:"MIT".into(),
        builder:"catalog.kedra#compiler".into(), runtime:"catalog.kedra#system".into(),
        source:SourceIntent::Files(BTreeMap::from([("hello.c".into(),Content::Inline {bytes:"#include <stdio.h>\nint main(void) { puts(\"original\"); return 0; }\n".into(),executable:false})])),
        build:Content::Inline {bytes:"mkdir -p \"$out/bin\"\n/usr/bin/gcc -O2 -ffile-prefix-map=\"$src\"=. \"$src/hello.c\" -o \"$out/bin/hello\"\n".into(),executable:false},
        env:BTreeMap::new(),build_deps:BTreeMap::new(),runtime_deps:BTreeMap::new(),files:BTreeMap::new(),replace_files:BTreeMap::new(),patches:Vec::new(),configs:BTreeMap::new(),timeout:600,
        exports:BTreeMap::from([("hello".into(),ExportIntent {kind:"command".into(),path:"bin/hello".into()})]),
    };
    let intent = Intent {schema_version:1,namespace:"example".into(),target:"demo".into(),foundation:"catalog.kedra#system".into(),packages:BTreeSet::from(["coreutils".into()]),remove:BTreeSet::new(),builders:BTreeMap::from([("catalog.kedra#compiler".into(),BTreeSet::from(["gcc".into()]))]),recipes:BTreeMap::from([(key.clone(),recipe)]),selected:BTreeSet::from([key])};
    let images = Images {foundation:runtime.clone(),builders:BTreeMap::from([("catalog.kedra#compiler".into(),builder.clone())])};
    let policy = Policy {namespace:"example".into(),packages:BTreeSet::from(["hello".into()]),builder_images:BTreeSet::from([builder]),runtime_images:BTreeSet::from([runtime]),source_objects:BTreeSet::new()};
    let lowered = language::lower(&intent,&BTreeMap::new(),&images,&policy).expect("checked typed lowering");
    println!("{}",serde_json::json!({"intent":intent,"catalog":lowered.catalog}));
}
