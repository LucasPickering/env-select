# Release Process

## Full CLI release

The release process for this tool is as follows:

- Check out the `master` branch
- Update the version number in `Cargo.toml` to `x.y.z`
- Run the following commands:

```
# Yes those are literal `v`s, as in "version"
git commit Cargo.toml Cargo.lock -m vx.y.z
git tag vx.y.z
git push --tags
```

- This should trigger the release job, which will:
  - Publish to crates.io
  - Create a draft release on GitHub
  - Attach build artifacts to the draft release
- Wait for the release CI jobs to finish
- Update the draft release with patch notes
- Publish the release
