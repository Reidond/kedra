# CLI contract for the first engine

Ordinary-user development commands. Results are JSON unless running a program;
runtime stdout/stderr and exit status remain observable. `--store` is explicit.

```text
sysroot build --store S --plan GRAPH --root NODE [--dry-run] [--rebuild]
sysroot store init --store S
sysroot store add-source --store S --source DIR
sysroot store add-image --store S --image sha256:HEX
sysroot store verify --store S --object ID
sysroot store closure --store S --object ID
sysroot store export --store S --object ID --output BUNDLE
sysroot store import --store S --bundle BUNDLE --expected-sha256 HEX
sysroot store pin --store S --object ID
sysroot store unpin --store S --object ID
sysroot store unpin-image --store S --image sha256:HEX
sysroot store gc --store S [--delete]
sysroot store recover --store S
sysroot run --store S --object ID --program RELATIVE [--timeout SECONDS] -- ARGS
sysroot profile switch --store S --name NAME --object ID --program RELATIVE -- ARGS
sysroot profile list --store S --name NAME
sysroot profile run --store S --name NAME [--timeout SECONDS] -- EXTRA_ARGS
sysroot profile rollback --store S --name NAME
sysroot develop --store S --name NAME [--program /DECLARED/IMAGE/PROGRAM]
                [--timeout SECONDS] -- ARGS
```

`build --dry-run` reads the explicitly supplied bounded graph only and invokes
pure graph resolution; it does not initialize/read the store or invoke Docker.
Sources/results/images are pinned before successful admission/build/import returns.
`gc` defaults to dry run. `recover` never removes an unrelated Docker container.
First execution platform is `aarch64-linux`. Graph serialization is the Rust API's
data protocol; no user configuration expression language is added.

Stable result fields: source admission `object`; image admission `image`; graph
planning `outputs` mapping node names to object IDs; build `outputs`, `built`,
`reused`, `reproduced`; export `sha256`; import `roots`; profile results include
selected generation/output; closure includes `objects` and `images`. Exact public
Rust method names belong to the library owner and must be published before CLI
integration, with no unilateral interface changes after the handshake.
