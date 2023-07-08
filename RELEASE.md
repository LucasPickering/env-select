# Release Process

**Requires [cargo-release](https://github.com/crate-ci/cargo-release)**

- Check out the `master` branch
- Run `cargo release <major|minor|patch>`
- The release job will be triggered automatically which will:
  - Create a draft release on GitHub
  - Attach build artifacts to the draft release
  - Publish to crates.io
- Update the draft release with patch notes
- Wait for the release CI jobs to finish
- Publish the release
- Update distribution packages
  - [brew](https://github.com/LucasPickering/homebrew-tap/blob/main/Formula/env-select.rb) - Update `version` and `sha256` sums
