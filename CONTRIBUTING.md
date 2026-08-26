# Contributing to TextScope

Thank you for considering a contribution. The project is a local-first,
deterministic text-forensics tool. Active development by the original
maintainer has ended, so contributions are welcome but no maintenance or
support commitment is promised.

## Before changing code

Install Rust with the `wasm32-unknown-unknown` target, Node.js/npm, and the
pinned `wasm-bindgen` CLI (`0.2.127`). Use the committed lockfiles. On
Windows, use `npm.cmd` for npm commands.

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
Set-Location apps/web
npm.cmd ci
npm.cmd run wasm:build
npm.cmd run typecheck
npm.cmd test
npm.cmd run build
npm.cmd run test:e2e
```

## Design and implementation rules

- Keep domain rules in `textscope-core`, the source of truth.
- Keep `textscope-wasm` as a thin interoperability bridge.
- Do not duplicate domain policy in React or TypeScript.
- Preserve deterministic, reproducible, content-preserving behavior.
- Treat every user input as untrusted and retain source evidence separately
  from extracted or Clean text.
- Do not add a backend, telemetry, analytics, remote text processing, or
  persistence of analyzed content.
- Evaluate every dependency for need, maintenance, license, security, WASM
  compatibility, and supply-chain impact.

## Tests and documentation

Add focused regression and preservation tests for every new domain rule. Update
the public documentation when behavior, limits, supported formats, or security
assumptions change. Run the full applicable gate set before opening a pull
request and include the commands and results.

## Pull requests

Keep changes small and explain the user-visible behavior, security impact,
test coverage, and any dependency or license effect. Do not include local
paths, credentials, user files, generated build output, or release archives.

## Attribution and provenance policy

Contributions must not add functionality intended to falsify attribution,
provenance, authorship, or document history. TextScope does not detect AI
generation, remove statistical LLM watermarks, or provide provenance-evasion
features. Metadata must remain evidence rather than unsupported authorship
claims.
