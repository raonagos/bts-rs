# Contributing to BTS

Thanks for helping improve **BTS**. This guide covers the local workflow and
the release process.

## Reporting bugs

Open an issue with:

- a clear description of the problem;
- the smallest reproducer you can write (ideally a failing test);
- the expected and actual behaviour;
- the Rust version and the enabled features.

## Suggesting improvements

Open an issue with the idea, why it is useful, and a small example if possible.
For API changes, describe the migration path for existing users.

## Pull requests

1. Fork the repository and create a branch from `master`:

   ```sh
   git clone https://github.com/raonagos/bts-rs.git
   cd bts-rs
   git checkout -b my-feature
   ```

2. Make the change and add tests. Bug fixes should include a regression test;
   new public items need documentation (the crate denies missing docs in CI).

3. Run the full local check before opening the PR:

   ```sh
   cargo fmt --all --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all-features
   RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
   ```

4. Open the pull request against `master`. CI runs the same checks plus a
   feature-matrix build.

## Rules

- Follow the existing style and keep the public API documented.
- Add a test for every behaviour change. Accounting changes need an invariant
  test in `src/engine/bts.rs` or `src/metrics.rs`.
- Do not add a runtime dependency unless it is optional and behind a feature.
- Keep the README and `docs/EXECUTION-MODEL.md` in sync with the engine.
- Write code, comments, tests, and documentation in English.

## Release process

Releases are published to [crates.io](https://crates.io/crates/bts-rs) by the
`Release` workflow when a `v*` tag is pushed.

1. Bump `version` in `Cargo.toml` and add the entry to [`CHANGELOG.md`](CHANGELOG.md).
2. Run the checks above, then commit the release.
3. Create and push the matching tag:

   ```sh
   git tag v1.1.0
   git push origin v1.1.0
   ```

4. The workflow checks that the tag matches the crate version, runs the tests,
   authenticates to crates.io with trusted publishing, and publishes with
   `--locked`.

The workflow uses crates.io trusted publishing, so no crates.io token is stored
in the repository. The `release` environment must be allowed by the crates.io
trusted-publisher configuration for this repository.

If trusted publishing is not configured, set a `CARGO_REGISTRY_TOKEN` secret
and replace the OIDC step with an environment variable; do not commit a token.

## License

By contributing, you agree that your contribution is licensed under the MIT
License.
