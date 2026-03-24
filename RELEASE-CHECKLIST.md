# Release Checklist

1. Update version `X.Y.Z` in `CHANGELOG.md`, write changelog.
- Update version `X.Y.Z` in `Cargo.toml`.
- `git tag -a vX.Y.Z -m "Release version X.Y.Z"`
- `git push origin --tags`
- `cargo publish --dry-run`
- `cargo publish`
- (Optional) Manually release version `X.Y.Z` on GitHub:
    1. `cross build --release --target aarch64-unknown-linux-gnu`
    * `cross build --release --target x86_64-pc-windows-gnu`
    * `cross build --release --target x86_64-unknown-linux-gnu`
    * Copy changelog / write release notes.
