# Development observations

## 2026-10-01

- Native artifact qualification observed that SIGKILL leaves the consumer's private
  verified context snapshot because destructor cleanup cannot run. Public cache
  recovery was tested with journal/tag state intact, after removing only the exact
  recorded process-owned temporary copy. Automatic crash-snapshot collection is
  unimplemented; any future cleanup must distinguish active snapshots and verify
  ownership without scanning/deleting arbitrary cache paths. Evidence:
  [.specs/nix-native-artifacts/verification.md](../.specs/nix-native-artifacts/verification.md).
