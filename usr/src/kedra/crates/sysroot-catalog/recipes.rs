use crate::{Catalog, Package, Recipe, Source, SourceOrigin};
use std::collections::BTreeMap;
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input};

// Observed through sysroot store add-source, 2026-10-02. Unpack the upstream
// archive, remove its enclosing directory and expand hardlinks into plain files;
// preserve the executable bit. The complete source tree is admitted unchanged.
pub const JQ_SOURCE: &str = "src-5d3f1f89c8b6b206fc3c133f24d90d6225d73cd4c6274ec610c4cacf9f44e2d2";
pub const SQLITE_SOURCE: &str =
    "src-f4837e042b248ee63fd02e3f51fd6c9b9e2ffdf34bfb57450dd22a10c67f4d65";

const JQ_BUILD: &str = r#"set -eu
mkdir /build/jq
cp -R "$1"/. /build/jq/
chmod -R u+w /build/jq
cd /build/jq
./configure --prefix="$2" --disable-docs --disable-maintainer-mode --disable-shared --enable-static --with-oniguruma=builtin CFLAGS='-O2 -ffile-prefix-map=/build=.'
make -j2
test "$(od -An -tx1 -N4 jq | tr -d ' \n')" = 7f454c46
mkdir -p "$2/bin" "$2/share/licenses/jq"
cp jq "$2/bin/jq"
chmod 755 "$2/bin/jq"
cp COPYING "$2/share/licenses/jq/COPYING"
cp vendor/oniguruma/COPYING "$2/share/licenses/jq/ONIGURUMA"
"#;

const SQLITE_LIBRARY_BUILD: &str = r#"set -eu
mkdir -p "$2/lib" "$2/include" "$2/share/licenses/sqlite"
/usr/bin/gcc -O2 -fPIC -shared -Wl,-soname,libsqlite3.so -ffile-prefix-map="$1"=. -DSQLITE_ENABLE_FTS5 -DSQLITE_ENABLE_RTREE -DSQLITE_ENABLE_MATH_FUNCTIONS -DSQLITE_ENABLE_COLUMN_METADATA "$1/sqlite3.c" -lpthread -ldl -lm -o "$2/lib/libsqlite3.so"
cp "$1/sqlite3.h" "$1/sqlite3ext.h" "$2/include/"
cp "$1/sqlite3.h" "$2/share/licenses/sqlite/sqlite3.h"
"#;

const SQLITE_BUILD: &str = r#"set -eu
mkdir -p "$3/bin"
/usr/bin/gcc -O2 -ffile-prefix-map="$1"=. -I"$2/include" "$1/shell.c" -L"$2/lib" -Wl,-rpath,"$2/lib" -lsqlite3 -lpthread -ldl -lm -o "$3/bin/sqlite3"
"#;

/// Application recipes with pinned upstream sources and caller-selected exact images.
/// Call `Catalog::resolve` with independent authority before opening the store.
pub fn builtin(builder_image: &str, runtime_image: &str) -> Catalog {
    let jq = node(builder_image, runtime_image, JQ_SOURCE, JQ_BUILD, 600);
    let library = node(
        builder_image,
        runtime_image,
        SQLITE_SOURCE,
        SQLITE_LIBRARY_BUILD,
        600,
    );
    let mut sqlite = node(
        builder_image,
        runtime_image,
        SQLITE_SOURCE,
        SQLITE_BUILD,
        300,
    );
    sqlite
        .inputs
        .insert("library".into(), Input::Node("sqlite-library".into()));
    sqlite.argv = vec![
        Argument::literal("/bin/sh"),
        Argument::literal("-c"),
        Argument::literal(SQLITE_BUILD),
        Argument::literal("sqlite-build"),
        Argument::input("source", ""),
        Argument::input("library", ""),
        Argument::output(""),
    ];
    sqlite.runtime_inputs.push("library".into());
    Catalog {
        namespace: "kedra".into(),
        packages: BTreeMap::from([
            (
                "jq".into(),
                Package {
                    version: "1.8.2".into(),
                    summary: "JSON transformation with bundled regular-expression support".into(),
                    license: "MIT and bundled notices in COPYING and vendor/oniguruma/COPYING"
                        .into(),
                    sources: vec![archive(
                        JQ_SOURCE,
                        "https://github.com/jqlang/jq/releases/download/jq-1.8.2/jq-1.8.2.tar.gz",
                        "71b8d6e8f5fe81f6c6d0d110e3892251f6ce76ed095abd315e26e6e1193af3af",
                    )],
                    recipe: Recipe {
                        graph: BuildGraph {
                            schema: 1,
                            nodes: BTreeMap::from([("jq".into(), jq)]),
                        },
                        root: "jq".into(),
                        program: "bin/jq".into(),
                    },
                },
            ),
            (
                "sqlite".into(),
                Package {
                    version: "3.53.4".into(),
                    summary: "SQLite database shell with a separate retained SQLite shared library"
                        .into(),
                    license: "blessing (public domain)".into(),
                    sources: vec![archive(
                        SQLITE_SOURCE,
                        "https://www.sqlite.org/2026/sqlite-autoconf-3530400.tar.gz",
                        "0e9483900e92cd5de8fd48d16bf9200145a61f7fd5be542a5ac81d8a9516eb9c",
                    )],
                    recipe: Recipe {
                        graph: BuildGraph {
                            schema: 1,
                            nodes: BTreeMap::from([
                                ("sqlite-library".into(), library),
                                ("sqlite".into(), sqlite),
                            ]),
                        },
                        root: "sqlite".into(),
                        program: "bin/sqlite3".into(),
                    },
                },
            ),
        ]),
    }
}

fn archive(object: &str, url: &str, sha256: &str) -> Source {
    Source {
        object: object.into(),
        origin: SourceOrigin::Archive {
            url: url.into(),
            sha256: sha256.into(),
        },
    }
}

fn node(
    builder: &str,
    runtime: &str,
    source: &str,
    script: &str,
    timeout_seconds: u64,
) -> BuildNode {
    BuildNode {
        builder_image: builder.into(),
        runtime_image: runtime.into(),
        inputs: BTreeMap::from([("source".into(), Input::Object(source.into()))]),
        argv: vec![
            Argument::literal("/bin/sh"),
            Argument::literal("-c"),
            Argument::literal(script),
            Argument::literal("catalog-build"),
            Argument::input("source", ""),
            Argument::output(""),
        ],
        env: BTreeMap::from([("SOURCE_DATE_EPOCH".into(), Argument::literal("0"))]),
        runtime_inputs: vec![],
        timeout_seconds,
    }
}
