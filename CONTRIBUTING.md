# Contributing

## Setup

Install the prerequisites listed in the [README](README.md#running-tests), then:

```bash
scripts/test.sh
```

If that passes on a clean checkout, you are set up. CI runs the same script.

## Before opening a pull request

- `scripts/test.sh` passes: formatting, clippy with warnings denied, Wasm build,
  and all tests.
- Contract behaviour changes come with a test named for what it protects, e.g.
  `a_rejected_job_refunds_the_client_once`, not `test_reject`.
- If a change affects an interface, invariant, or trust assumption, update the
  matching file in `specs/` in the same PR.
- Constants that must match an upstream contract or standard cite their source
  at the point of use.

## Review rules

- Changes under `contracts/`, `scripts/deploy-*`, or `deployments/` move or guard
  funds. They need **two maintainer approvals**.
- Everything else needs one.
- Key handling, evaluator policy, release scripts, and signing logic are
  maintained by core maintainers. Open an issue to discuss before working on them.

## Commit identity

Commit with an email linked to your GitHub account so your contributions are
credited.
