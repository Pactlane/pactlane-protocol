# bindings

Generated TypeScript clients for Pactlane contracts. The `pactlane` application
repository consumes these as pinned, versioned packages (`@pactlane/commerce`,
`@pactlane/evaluation-policy`) and never edits them by hand.

```bash
scripts/build.sh
scripts/generate-bindings.sh   # writes packages/bindings/<contract>/
```

Local output is gitignored. On a version tag, `.github/workflows/release.yml`
builds each client as `@pactlane/<contract>` (for example `@pactlane/commerce`)
at the tag's version. It attaches the packed tarballs to the GitHub release, next
to the Wasm they were generated from and a `SHA256SUMS` file. Install a release
tarball directly; publishing to the npm registry comes later.
