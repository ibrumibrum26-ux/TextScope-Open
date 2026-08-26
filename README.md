# TextScope

TextScope is a local-first tool for forensic text inspection and deterministic,
content-preserving canonicalization. It helps a user understand what a text or
local document contains, apply an explicit cleaning policy, and verify the
result without sending the input to a server.

## Project status

Version `0.1.0` contains the implemented local workflow and the TextScope Clean
Standard (TCS) `0.1.0`. Active development by the original maintainer has
ended; the code is being made available to the community. Contributions are
welcome, but this project makes no promise of ongoing maintenance, an SLA, or
support. Publication and deployment require a separate human review.

## What it does

TextScope accepts the following local inputs:

- pasted Unicode text;
- an explicit, one-shot rich-clipboard read (`text/plain` and `text/html`);
- literal `.txt`, `.md`, and `.markdown` files;
- inert forensic inspection of `.html`/`.htm`, `.docx`, and `.pdf` files.

The Rust core records encoding evidence, Unicode coordinates, structural
observations, metadata, references, and provenance evidence where the format
provides it. It keeps source evidence separate from extracted text and from
the optional Clean result.

The workflow is deterministic:

```text
input → strict decode/forensics → Scan → Reveal → explicit TCS Clean
      → independent rescan → Verification Report
```

Clean is content-preserving by contract. It is versioned, manifest-backed, and
never rewrites the source file. Attribution is not inferred from metadata.

## Architecture

```text
Rust Core (source of truth)
        ↓
thin wasm-bindgen bridge
        ↓
TypeScript / React / Vite browser UI
```

The application is local-first: the browser reads the selected input, the
WASM bridge transports bounded bytes, and the Rust core performs the domain
analysis. There is no backend, telemetry, analytics, remote text processing,
or required network service.

## Privacy and security model

Input bytes, decoded text, clipboard representations, findings, and Clean Text
remain in browser and Rust/WASM memory. The application does not upload,
persist, log, or export analysis data. Clipboard access happens only after an
explicit user action and is bounded before WASM.

HTML, DOCX, and PDF are untrusted evidence. They are parsed as inert data:
scripts, macros, URLs, attachments, OCR, browser DOM interpretation, and
filesystem access are not invoked. Resource limits reject oversized or
malformed input rather than silently truncating it.

## Limitations and non-goals

- The UI input limit is currently 10 MiB.
- Source files are never rewritten; Document Clean and Developer Clean are
  reserved for future specifications.
- OCR and C2PA signature/trust validation are not implemented.
- TextScope does not detect AI-generated text, remove statistical LLM
  watermarks, or provide a tool for evading provenance or attribution.
- Metadata is evidence, not proof of authorship or intent.
- The validated browser point for this release is Microsoft Edge Stable on a
  local Vite origin; other browsers are not promised by this release.

## Development

The repository uses a Cargo workspace and the committed lockfiles. Install a
current Rust toolchain with the `wasm32-unknown-unknown` target, Node.js and
npm, and the pinned `wasm-bindgen` CLI (`0.2.127`). Then, from the repository
root:

```powershell
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo audit
```

On Windows, run the web commands with `npm.cmd`:

```powershell
Set-Location apps/web
npm.cmd ci
npm.cmd run wasm:build
npm.cmd run typecheck
npm.cmd test
npm.cmd run build
npm.cmd run test:e2e
npm.cmd audit
npm.cmd audit --omit=dev
```

The canonical validation scope is 283 tests: Rust Core 127, Rust WASM 29,
frontend 65, and Microsoft Edge E2E 62. The E2E suite requires a local Edge
installation and never sends analyzed content to a remote service.

## Contributing and security reports

See [CONTRIBUTING.md](CONTRIBUTING.md) for the change and testing policy and
[docs/SECURITY.md](docs/SECURITY.md) for the threat model and reporting
guidance.

## License

TextScope is licensed under the [Apache License, Version 2.0](LICENSE).
Dependency licenses and the current inventory are documented in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md). This inventory is provided
for engineering review and is not legal advice.
