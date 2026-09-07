# Required release-negative cases

Every result records source/run, exact tools and digests, expected/actual exit,
running/staged state before/after, and safe recovery. These are planned tests.

| Input or event | Expected boundary |
|---|---|
| Unsigned or wrong-key OCI image | bootc enforcing path rejects it |
| Allowed key, other registry repository | repository identity rejects it |
| Valid signature, other enrolled target/architecture | release eligibility rejects it |
| Changed image digest or home payload in metadata | metadata/image binding rejects it |
| Correctly signed but not promoted candidate | no routine deployment |
| Missing attachment after an image copy | fail, never skip verification |
| Slow old build finishing after newer release | channel does not regress |
| Replay of valid old metadata | not mistaken for newest forward update |
| User passes --verified or supplies writable manifest | helper verifies independently |
| PR changes signer/check scripts or forges artifact | no production signing authority |
| Candidate image contains executable attack payload | signing job never runs it |
| Key rotates while a machine stays offline | explicit supported trust transition/recovery |
| Registry GC removes tags | retained required digests/signatures remain usable |
| Network disappears after staging | documented local boot/recovery path |

Store disposable fixtures outside production namespaces. Never use the user's
SSH identity as a release key. Public signature material is safe to commit;
production private keys and passphrases are not. Attestation and branch protection
are complementary controls, not substitutes for machine-side signature policy.
