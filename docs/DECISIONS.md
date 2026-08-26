# Public architecture decisions

This is a sanitized public summary of the technical decisions behind
TextScope. Operational history and private release details are intentionally
omitted; the summary does not change the implementation.

## 1. Keep the Rust core authoritative

Unicode scanning, classification, normalization, canonicalization, diffing,
verification, and format-specific forensics live in `textscope-core`. The
frontend does not duplicate domain rules.

## 2. Use a Cargo workspace and a thin WASM bridge

`textscope-wasm` exposes the core through a small `wasm-bindgen`/JSON boundary.
The bridge transports data and serializes results; it does not own product
policy. The dependency direction is `web → textscope-wasm → textscope-core`.

## 3. Keep the product local-first

The browser reads local input and the Rust/WASM runtime analyzes it in memory.
There is no backend, upload path, telemetry, analytics, or remote text
processor. Clipboard access is explicit, one-shot, and bounded.

## 4. Make decoding and provenance explicit

Text and file bytes are decoded strictly with documented encoding precedence.
Unicode coordinates and source ranges are retained where they are reliable;
ambiguous or unavailable ranges are reported rather than invented.

## 5. Treat documents as inert evidence

HTML, DOCX, and PDF are parsed with bounded, read-only adapters. Scripts,
macros, URLs, attachments, OCR, browser DOM interpretation, filesystem I/O,
and source rewriting are outside the adapters' responsibilities.

## 6. Keep the cleaning contract versioned

The TextScope Clean Standard (TCS) is explicit registry data, currently
`0.1.0`. Clean produces a transformation manifest, preserves source evidence,
rescans independently, and checks determinism, idempotence, and content
preservation.

## 7. Prefer small, reviewed dependencies

Each dependency is reviewed for maintenance, license, security, WASM
compatibility, and supply-chain impact. Lockfiles are committed. The current
license inventory is in [THIRD_PARTY_LICENSES.md](../THIRD_PARTY_LICENSES.md).

## 8. Keep browser adapters narrow

The React/TypeScript layer owns presentation and browser APIs. File reads are
bounded before WASM; generated bindings are reproducible; the web package
remains private to prevent accidental npm publication, while Rust crates keep
`publish = false` until registry distribution is deliberately reviewed.

## 9. Use deterministic format-specific paths

TXT and Markdown are literal text. HTML combines strict decoding, source
tokenization, and tolerant structure without executing content. DOCX uses
bounded ZIP/OOXML inspection. PDF uses bounded object and content-stream
inspection without rendering or executing actions.

## 10. Make provenance evidence non-attributive

Format-specific metadata is preserved and exposed through a shared evidence
contract. Observations and declarations are not presented as authorship proof;
conflicts remain visible and attribution is unsupported without independent
evidence.

## 11. Evaluate, but do not silently add, C2PA

C2PA signature and trust validation is not implemented. Adding it would require
an explicit manifest format, trust/revocation policy, bounded parser, and a
separate security and WASM review.

## 12. Validate the browser boundary

Rust tests, TypeScript tests, Vite builds, supply-chain audits, and real Edge
E2E tests are treated as release gates. Security review includes network,
persistence, DOM/XSS, unsafe-code, secret, and machine-path checks.

## 13. License the project explicitly

This public copy uses the Apache License 2.0. Third-party components retain
their own terms; see the dependency inventory and review its limitations
before redistribution.
