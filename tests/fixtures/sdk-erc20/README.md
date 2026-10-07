# ERC-20 Transfers

The reference SuperQuery project: index `Transfer` events from a single ERC-20
contract.

```bash
superquery validate   # check manifest, schema and ABIs — offline, fast
superquery codegen    # generate src/generated/ from schema.graphql + abis/
cargo check           # confirm the mapping compiles against generated code
superquery build      # compile to WASM and bundle dist/
```

## Layout

| Path | What it is |
|---|---|
| `project.yaml` | Which chain, which contract, which handlers |
| `schema.graphql` | The entities this project stores |
| `abis/ERC20.json` | The contract ABI, used to generate typed events |
| `src/lib.rs` | The mapping — one function per handler |
| `src/generated/` | Generated; never edit, never commit |

## Editing

Change `schema.graphql` or an ABI, re-run `superquery codegen`. The generated
code is deterministic, so a no-op run rewrites nothing.
