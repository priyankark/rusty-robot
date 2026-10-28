# Releasing

The public Cargo package is `rusty-robot-desktop`, owned by `priyankark`.
GitHub releases contain its source archive, the Node package with seven prebuilt
binaries, and SHA-256 checksums. Cargo and npm publication are separate actions;
creating a GitHub release does not upload to either registry.

1. Update the core, media, Node bridge, and Node package versions together. Update
   both Cargo lockfiles, the changelog, Cargo installation examples, and validation
   evidence. Keep npm installation instructions on the last published npm version
   until the new npm package is actually available.
2. Require green Rust contracts on Linux/macOS/Windows, MSRV, reference audit,
   package verification, benchmark/report consistency, and documentation generation.
   Native desktop checks must cover affected platforms; record hardware omissions.
3. Run `cargo publish --dry-run --locked` and inspect `cargo package --list` and
   the archive. The media/Node workspace crates remain `publish = false`.
4. Merge the release preparation. Push an annotated `vX.Y.Z` tag at that commit,
   matching Cargo.toml. Never move a published release tag.
5. The tag workflow reruns three-OS tests, builds and installs all seven native
   targets, and creates a GitHub release with `.crate`, `.tgz`, and `SHA256SUMS`.
6. From a clean checkout of the tag, run `cargo publish --locked`. Use the existing
   registry publishing identity; do not put credentials in source or logs.
7. Verify the registry version and run `python3 scripts/test-registry.py --version
   X.Y.Z`. Dispatch `registry-smoke.yml` with that version to test a fresh crates.io
   consumer on Linux, macOS, and Windows. This checks the registry artifact, not
   a local path dependency.
8. When publishing npm, publish the exact verified release tarball, then verify a
   clean install from npm before updating its installation examples.

CI deploys the benchmark site and rustdoc after required checks pass on main.
Third-party actions are pinned to commits. Local macOS input tests need an
unlocked desktop and permission grants; the registry smoke test does not inject
input unless explicitly invoked with `--desktop`.
