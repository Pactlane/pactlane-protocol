# bindings

Generated TypeScript clients for Pactlane contracts. The `pactlane` application
repository consumes these as a pinned, versioned package (`@pactlane/contracts`)
and never edits them by hand.

```bash
scripts/build.sh
scripts/generate-bindings.sh   # writes packages/bindings/<contract>/
```

Local output is gitignored. Bindings are published from CI on a version tag, built
from the same Wasm whose hash is recorded in `deployments/`.
