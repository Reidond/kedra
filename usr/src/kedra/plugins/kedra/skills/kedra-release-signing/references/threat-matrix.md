# Required release-negative cases

Every result records source, exact tools and digests, expected/actual exit,
running/staged state before/after, and safe recovery. "VM" marks cases
`usr/src/kedra/tests/vm/ghcr-update/run.sh` implements (its evidence `scope.json` lists
`implemented_cases` and `not_run_cases`); the others are planned.

| Input or event | Expected boundary | Coverage |
|---|---|---|
| Unsigned or wrong-key OCI image | bootc enforcing path rejects it before any layer | VM |
| Allowed key, other registry repository | repository identity rejects it | VM |
| Valid signature, other target/architecture/channel | helper identity check rejects it | VM |
| Identity label with unknown fields | strict parsing refuses it | VM (malformed identity) |
| Configuration not the one the hashed manifest names | helper refuses the identity | planned |
| Valid older signed image (replay or downgrade) | rank check against the booted identity refuses it | VM |
| Same rank, different digest | equivocation refused | VM |
| Missing signature attachment after an image copy | fail, never skip verification | VM |
| Identity mismatch after boot | forward work blocked, retained rollback available | VM |
| Correctly signed but not promoted candidate | no routine deployment | planned |
| Slow old build finishing after newer release | channel does not regress | planned (tag race not-run) |
| User passes --verified or supplies writable manifest | helper verifies independently | planned |
| PR changes signer/check scripts or forges artifact | no production signing authority | planned |
| Candidate image contains executable attack payload | signing job never runs it | planned |
| Key rotates while a machine stays offline | explicit supported trust transition/recovery | planned |
| Registry GC removes tags | retained required digests/signatures remain usable | planned |
| Registry unreachable during check | check fails, nothing changes | VM (offline) |
| Network disappears after staging | documented local boot/recovery path | planned |

Store disposable fixtures outside production namespaces. Never use the user's
SSH identity as a release key. Public signature material is safe to commit;
production private keys and passphrases are not. Attestation and branch protection
are complementary controls, not substitutes for machine-side signature policy.
