# Available-environment discovery follow-up

Date: 2026-09-07. Agent: Codex. Base: main at
64c84498c898e61ff370d9f9f1ce30cbd3d00613, with local R03 changes.
Codex CLI 0.153.4; Claude Code 2.1.263; Windows x86_64.
Overall R11 remains blocked. No plugin was installed or registered.

| Probe | Actual | Status |
|---|---|---|
| Current Codex session skill catalog | No Kedra skills advertised | blocked: automatic discovery unavailable in this session |
| Canonical manual reads | kedra-context, kedra-rust-workspace, rust-router, domain-cli, m06-error-handling, m07-concurrency and relevant Kedra skills readable and used | pass: file access only |
| Supporting file from root and crate cwd | kedra-home/references/state-model.md readable via correct relative path | pass: file access only |
| codex plugin list --marketplace kedra-local --available --json (root and crates/sysroot-core) | installed=[], available=[], exit 0 | blocked: no discovered Kedra entry |
| Codex app-server skills/list, fresh empty fixture CODEX_HOME, root and crate cwds | Six built-in skills each; zero skills from plugins/kedra; errors=[] | blocked: repository plugin not automatically loaded |
| Claude plugin validate ./plugins/kedra | Validation passed, exit 0 | pass: manifest authoring only |
| Claude plugin validate ./.claude-plugin/marketplace.json | Validation passed, exit 0 | pass: marketplace authoring only |
| Ordinary tree/static wiring | 30 canonical SKILL.md files; no plugin reparse points; no tracked mode-120000/160000 entries; manifests point at same skill tree | pass: static review only |
| Claude model session with --plugin-dir | No interactive authenticated fixture session launched | not-run |
| Codex loaded-plugin model session / personal conflicting skills / moved-offline cases | No registration/installation or model run performed | not-run |
| Editor/rust-analyzer UI and diagnostics | No editor session exercised | not-run |
| Distribution/license review | Existing missing upstream LICENSE text gap unchanged | blocked |

The read-only Codex app-server probe used its own generated empty profile under
ignored target/r11-profile-<unique>, without copying credentials or altering the
personal profile. No turn/thread/model request was sent. Protocol schemas came
from the installed binary via `codex app-server generate-json-schema --out
target/r11-schema`; the schema supports these requests:

```json
{"id":1,"method":"initialize","params":{"clientInfo":{"name":"kedra-r11-probe","version":"1"}}}
{"method":"initialized"}
{"id":2,"method":"skills/list","params":{"cwds":["<absolute-checkout>","<absolute-checkout>/crates/sysroot-core"],"forceReload":true}}
```

To reproduce, start `codex app-server` with process-scoped CODEX_HOME pointing to
a newly created empty target directory, send initialize and await its response,
then send initialized and skills/list over stdio. Read the matching id=2 response
and close stdin. Do not install the repository plugin globally to obtain a pass.
The recorded response summary excludes irrelevant built-in metadata:

```json
[
  {"cwd":"<checkout>","totalSkills":6,"kedraSkills":[],"errors":[]},
  {"cwd":"<checkout>/crates/sysroot-core","totalSkills":6,"kedraSkills":[],"errors":[]}
]
```

No intentional personal/global configuration writes were made. This is not an
OS-level write-trace or a proof of complete runtime profile isolation (R05).
The available Codex can continue by reading canonical files explicitly. Native
loading remains a separate opt-in test; restarting or manifest validity alone
has not been demonstrated to fix discovery. Repository instructions and source
review still enforce flat Rust layout independently of plugin availability.

Primary documentation consulted: [OpenAI plugins](https://learn.chatgpt.com/docs/plugins).
The installed 0.153.4 CLI help/schema and actual responses above establish this
environment's results; documentation support does not constitute a discovery pass.
