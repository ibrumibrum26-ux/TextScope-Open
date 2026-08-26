# Changelog

## [0.1.0] - 2026-08-22

### Added

- Deterministic Unicode scanner and Reveal output.
- Versioned TCS Clean Engine 0.1.0 with transformation manifest, independent
  rescan, content-preservation diffs, and Verification Report.
- Paste Text, Rich Clipboard, TXT, Markdown, HTML, DOCX, and PDF local-input
  paths with forensic observations and provenance/metadata evidence.
- Local-first browser processing through the Rust core and WebAssembly bridge.
- Security and resource hardening, including bounded parsing and accessibility,
  responsive, cross-format, clipboard, and real-browser validation.

### Known limitations

- OCR and C2PA are not implemented.
- Source files are never rewritten; Document Clean and Developer Clean remain
  reserved.
- The current local browser input limit is 10 MiB.
- The officially validated browser support point for this release is Microsoft
  Edge Stable.
