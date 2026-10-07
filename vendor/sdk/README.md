# SDK contract snapshot

Unmodified Rust sources from `blockSuperquery/superquery-sdk`, commit
`b15709b23ce77cd88ab856cbb84ff34ed09f9512`, crates `types` and `manifest`.
Apache-2.0 license retained in LICENSE. Cargo manifests are adapted to standalone
path dependencies; source ownership remains with the SDK. Refresh both crates
together from an SDK revision, never add node-specific wire fields here.

This snapshot keeps standalone node checkouts reproducible while shared crates
are unpublished. Node project policy belongs in `crates/config/src/project.rs`.
