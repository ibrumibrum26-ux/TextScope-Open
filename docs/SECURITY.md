# TextScope security and privacy

## Boundary and data handling

TextScope is local-first. Raw files, decoded text, clipboard representations,
findings, and Clean Text stay in the browser process and Rust/WASM memory.
There is no backend, upload endpoint, telemetry, analytics, remote text
processing, or required network client. The application does not use browser
storage, cookies, or a filesystem writer for analysis data.

Clipboard access is explicit and one-shot. The app does not read at startup,
poll in the background, read clipboard history, write the clipboard, or log
clipboard bytes. Unsupported advertised MIME types are listed without reading
their contents, and the aggregate payload is checked before it reaches WASM.

## Untrusted input model

User-controlled text and local files are untrusted evidence. HTML, DOCX, and
PDF are parsed as inert data. TextScope does not render supplied HTML, execute
scripts or macros, follow URLs, open attachments, invoke OCR, use a browser DOM
interpreter for the supplied document, or access the filesystem from Rust.

All adapters apply deterministic resource limits. Oversized, malformed, or
unsupported input is rejected with a controlled error rather than silently
truncated or decoded with replacement characters. Original bytes/text remain
separate from extracted text and the optional TCS Clean result.

## Provenance semantics

The evidence contract distinguishes observations, declarations, verification,
and attribution. Stored HTML generator, DOCX creator, and PDF author/producer
fields are declarations only. Conflicting observations remain visible, and
attribution is unsupported without independent evidence. TextScope does not
infer authorship, intent, or AI generation.

## Supply-chain and code review

The Rust workspace forbids unsafe product code. Lockfiles are committed.
`cargo fmt`, `cargo check`, `cargo clippy`, `cargo audit`, `npm audit`, and
`npm audit --omit=dev` are read-only review gates; automatic vulnerability
fixes are not part of the release process. Production code is reviewed for
network APIs, persistence, XSS/DOM sinks, unsafe calls, panic-oriented paths,
secret leakage, and machine-specific paths.

## Reporting a vulnerability

This copy does not define a private security-advisory address. Do not include
secrets, personal data, or a weaponized proof of concept in a public issue. A
maintainer can enable GitHub Private Vulnerability Reporting before sensitive
reports are accepted. For a safe, non-sensitive report, provide the affected
version, a minimal reproduction, expected behavior, and impact while omitting
the original text or file contents.

## Scope

Security review covers the local browser/WASM boundary and the bounded parsers
included in this repository. It does not promise security of a future hosting
provider, browser, operating system, third-party dependency, or deployment
configuration. Review the dependency inventory and the deployment guidance
before distributing a build.
