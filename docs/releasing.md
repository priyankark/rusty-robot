# Releasing

GitHub releases distribute the verified Cargo source package. This repository
currently uses Git dependencies; a GitHub release does not imply crates.io or npm
publication. The `release.yml` workflow does not require registry credentials.

1. Update Cargo version, lockfile, changelog, install tag, and dated validation.
2. Require green Rust contracts (Linux/macOS/Windows), MSRV, reference audit,
   package verification, benchmark consistency, and documentation generation.
3. Run native tests on platforms affected by input/capture changes and record
   omissions; do not infer GUI correctness from compile success.
4. Run `cargo package --locked`, inspect `cargo package --list`, and review the
   archive. Check the published benchmark data contains no private captures.
5. Push an annotated `vX.Y.Z` tag matching Cargo.toml. The release workflow checks
   the version, reruns tests on three OSes, verifies the package, and creates a
   GitHub prerelease for 0.x with the `.crate` archive and SHA-256 checksum.

CI deploys the benchmark site and rustdoc to GitHub Pages only after its required
checks pass on main. Third-party actions are pinned to commits and Dependabot
proposes updates. Native desktop jobs are manually dispatched because they
require a real session and may need permission grants.

For a future crates.io release, first confirm name ownership and configure a
registry publishing identity; use `cargo publish --dry-run --locked` before the
actual publish. Registry uploads are not part of the current GitHub workflow.
