# TextScope v0.1.0 release-readiness checklist

This public-facing checklist describes the implemented local-first scope. It is
not a promise of future features, maintenance, or hosting. Publication and
deployment are separate decisions requiring human review.

## Scope

Implemented inputs are Paste Text, Rich Clipboard, TXT, Markdown, HTML, DOCX,
and PDF. Each path retains source evidence, produces decoded or extracted text,
and converges on Rust Scan, Reveal, explicit TCS 0.1.0 Clean, an independent
rescan, and a verification report. Source evidence is never rewritten.

Attribution remains unsupported without independent evidence. Document Clean
and Developer Clean are reserved. C2PA is evaluated but not implemented.

## Quality gates

The following commands are the reproducible review gates for a clean checkout:

| Area | Gate |
| --- | --- |
| Rust | `cargo fmt --all -- --check` |
| Rust | `cargo check --workspace --all-targets` |
| Rust | `cargo test --workspace --all-features` |
| Rust | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Rust/WASM | target checks and `npm.cmd run wasm:build` |
| Web | `npm.cmd ci`, typecheck, unit tests, and Vite build |
| Browser | 62 real Microsoft Edge E2E tests on the local origin |
| Supply chain | `cargo audit`, `npm.cmd audit`, and `npm.cmd audit --omit=dev` |
| Hygiene | `git diff --check` and secret/path review |

The canonical test reconciliation is 283: Rust Core 127, Rust WASM 29,
frontend 65, and Edge E2E 62. A release candidate is acceptable only when all
applicable gates pass in the exact environment being reviewed.

## Known limitations

- OCR and cryptographic C2PA validation are not implemented.
- The browser input limit is 10 MiB.
- Source documents are never rewritten.
- Metadata is evidence, not proof of authorship or intent.
- TextScope does not detect AI generation, remove statistical LLM watermarks,
  or provide provenance-evasion functionality.
- Microsoft Edge Stable on a local Vite origin is the validated browser point
  for this release; other browsers require their own review.

## Community hand-off

Active development by the original maintainer has ended and the code is being
made available to the community. Contributions are welcome. No maintenance
commitment, SLA, or support promise is made by this project.
