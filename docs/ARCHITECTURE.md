# TextScope architecture

## Dependency direction

```text
React UI
  ↓
TypeScript WASM wrapper
  ↓
wasm-bindgen generated boundary
  ↓
textscope-wasm
  ↓
textscope-core
```

Dependencies point toward the core. The core does not know about the WASM or UI
layers.

The file-ingestion path preserves the byte/Unicode boundary explicitly:

```text
Browser File API
  ↓ File.arrayBuffer(), only after the 10 MiB UI guard
raw source bytes (Uint8Array)
  ↓
wasm-bindgen byte slice
  ↓
Rust Text Ingestion
  ↓
encoding inspection and strict decode
  ↓
decoded Unicode text + source-byte mapping
  ↓
scanner / reveal / TCS clean / rescan / verify
```

The browser owns physical file reading. The WASM bridge owns only transport and
serialization. The Rust core receives `&[u8]` and never opens a file.

Rich Clipboard is an explicit browser adapter and never runs in the background:

```text
User clicks Read Clipboard or pastes
  ↓
navigator.clipboard.read()/readText() or ClipboardEvent.clipboardData
  ↓ 10 MiB aggregate guard before WASM
advertised text/plain + text/html + unread unsupported MIME labels
  ├→ text/plain bytes → strict Rust text ingestion → scan
  ├→ text/html bytes → inert Rust HTML adapter → structure + extracted text
  └→ deterministic comparison (equivalent / technical / content / review)
        ↓
      selected text → Reveal / TCS Clean / independent verification
```

Clipboard data is read only after explicit user action, is not polled, written,
persisted, logged, or sent to a network. The HTML representation never enters
`DOMParser`, `innerHTML`, an iframe, or another browser interpretation surface.

HTML uses a specialized branch after raw-byte inspection:

```text
Browser HTML File (.html / .htm)
  ↓ 10 MiB pre-read guard
Uint8Array → &[u8]
  ↓
strict BOM / UTF-8 / BOM-authorized UTF-16 decode
  ↓
WHATWG source tokenizer → observed spans + charset declarations
  ↓
HTML5 tree builder → tolerant parsed structure + recovery observations
  ↓
lean structural model
  ├→ metadata declarations
  ├→ external-reference declarations (never resolved)
  ├→ comments and active/technical text channels
  └→ extracted document text
        ↓
      scanner / reveal / TCS clean / rescan / verify
```

PDF has a binary-first branch that never renders or rewrites the source:

```text
Browser PDF File (.pdf only)
  ↓ 10 MiB pre-read guard
Uint8Array → &[u8]
  ↓
bounded lopdf parse with decompression and traversal limits
  ↓
header/version/EOF/startxref/xref/revision/encryption evidence
  ↓
objects/streams/page tree/resources/fonts/images
  ├→ Info/XMP metadata and conflicts
  ├→ annotations/actions/URI references/JavaScript/attachments/forms/XFA/signatures
  └→ content-stream text + page/stream/font provenance
        ↓
      scanner / reveal / TCS clean / rescan / verify
```

The PDF branch has no renderer, browser PDF viewer, OCR engine, JavaScript
runtime, URI client, attachment opener, filesystem extraction, cryptographic
signature verifier, or source rewriter. Active content is escaped structural
data only; no action is invoked.

`scraper`/`html5ever` owns HTML5 tree construction. `html5gum` independently
tokenizes the already strictly decoded source and supplies reliable token
spans. The two views are intentionally not conflated: source-observed element
records carry token ranges, while implied or recovered tree nodes have no
invented source location. A parsed text node gains source spelling and ranges
only when the next contiguous token sequence decodes to that exact node text.

DOCX has a package-first branch that never writes archive entries:

```text
Browser DOCX File (.docx only)
  ↓ 10 MiB pre-read guard
Uint8Array → &[u8]
  ↓
raw central-directory evidence + bounded zip reader
  ↓
OPC required parts → content types → relationships (never resolved externally)
  ↓
bounded, strict XML decode → read-only XML tree
  ↓
part inventory + observed metadata + WordprocessingML structure
  ├→ comments / revisions / headers / footers / notes
  ├→ inert media, embedded, active-related, and custom XML evidence
  └→ extracted body text + segment provenance
        ↓
      scanner / reveal / TCS clean / rescan / verify
```

The `zip` crate owns ZIP parsing, decompression, CRC verification, ZIP64
support, and safe memory-backed entry reading. A deliberately narrow raw
central-directory reader records raw entry names and detects duplicates before
the public indexed archive view can hide them; it does not decompress data or
replace the ZIP implementation. `roxmltree` owns read-only XML tree parsing,
positions, and node limits with DTD processing disabled. TextScope owns only
OPC/WordprocessingML traversal and explicit limits.

## `textscope-core`

The core owns all deterministic domain behavior: Unicode interpretation,
scanning, classification, canonicalization, diffing, and verification. It must
remain independent of browsers, Node.js, React, and presentation concerns.

Browser values, DOM types, JavaScript callbacks, UI state, and transport
concerns must not cross into the core. Keeping this boundary platform-neutral
makes its rules directly testable and reusable.

The pasted-text domain pipeline is composed of six core-owned parts:

```text
Scan Engine
  ↓
Clean Policy Engine ← TCS Registry
  ↓
Transformation Engine
  ↓
Independent Rescan
  ↓
Verification Engine
```

- **Scan Engine:** creates character records, conservative findings,
  normalization observations, and technical reveal output.
- **TCS Registry:** defines TCS version 0.1.0, versioned rule metadata, supported
  and reserved profiles, and each profile's explicit active-rule list.
- **Clean Policy Engine:** accepts one typed profile and selects only registry
  rules authorized for that profile.
- **Transformation Engine:** detects non-overlapping changes against the
  original text, orders them by original location, records a manifest, and
  reconstructs Clean Text by replaying that manifest.
- **Independent Rescan:** calls the Scan Engine again with Clean Text; it never
  reuses the original scan as the clean scan.
- **Verification Engine:** checks manifest replay, profile authorization,
  declared changes, rescan, expected and preserved findings, determinism, and
  idempotence before assigning a verification status.

The **Text Ingestion Engine** precedes that pipeline for file inputs. It selects
only strict UTF-8 or BOM-authorized UTF-8/UTF-16, validates surrogate structure,
and emits one source mapping per decoded Unicode scalar. It does not perform
lossy replacement, charset guessing, line-ending conversion, cleaning, or I/O.
Strong alternating-NUL patterns resembling BOM-less UTF-16 are rejected as
ambiguous without claiming or decoding an encoding.

The **HTML Analysis Engine** is a sibling of literal text ingestion. It reuses
only the strict decoder and scalar source mappings. HTML-specific meta evidence,
HTML5 parsing, structural findings, reference classification, text extraction,
and resource limits do not enter the TXT/Markdown adapter. The engine has no
renderer, JavaScript runtime, CSS engine, URL resolver, filesystem API, or
network client.

The HTML output is a lean ordered inspection model rather than a serialized
DOM: document summary, observed start tags and attributes, metadata,
text segments, comments, references, findings, parser recoveries, and explicit
text channels. It records source ranges as zero-based half-open decoded UTF-8
and raw-byte coordinates. If a raw boundary cannot be proven from the strict
scalar map, the raw range is `None`.

The **DOCX Analysis Engine** is another sibling. It accepts bytes only and has
no filesystem extraction, Office/COM automation, macro/OLE/ActiveX execution,
renderer, URL client, DNS, or shell. It validates the standard main-document
content type and root relationship, inventories package parts, parses all
relationship parts, and preserves external targets as inert declarations.
Parts and relationships are never treated as permission to access a resource.

Every `w:t` fragment is retained separately with part, channel, structural
indices, fragment kind, `xml:space`, explicit hidden marker, revision context,
and a decoded XML range when available. Strict UTF-8/UTF-16 decoding maps that
range back to the uncompressed part only when provable. No compressed package
range is invented. Derived channel strings are assembled deterministically;
only the body enters the default Unicode scanner and explicit TCS pipeline.
The source byte slice is borrowed/read and never mutated.

The **PDF Analysis Engine** is a sibling that accepts an untrusted byte slice
and returns a bounded forensic model. It records parser/header evidence,
object and stream inventory, xref/revision observations, page tree and
resource references, metadata, annotations, actions, external references,
embedded-file declarations, forms/XFA, signature dictionaries, images, and
font/ToUnicode status. Content streams are decoded only within explicit
per-stream, aggregate, operation, reference, and result limits. Text segments
retain page, content-stream, font, operation, and source-object references;
raw offsets are omitted when xref data cannot prove them. No replacement
character or OCR result is fabricated when a font mapping is unavailable.

The **Provenance Evidence Model** is shared across HTML, DOCX, PDF, and
clipboard adapters. It records an exact subject/value, adapter source,
`EvidenceKind`, `EvidenceLevel` (Observation/Claim/Verification/Attribution),
and `VerificationState` (Observed/Declared/Verified/NotVerified/Unsupported/
Conflict). Format-specific metadata remains intact; the common list is an
additional lossless view. Attribution is never inferred from a stored author,
creator, generator, producer, or clipboard representation.

File-byte ranges, decoded-string UTF-8 offsets, decoded UTF-16 code-unit
offsets, and code-point indexes are separate coordinate systems. Source ranges
are zero-based and half-open. A byte-order mark is source metadata; it is not a
character record. The existing scanner contract and its decoded UTF-8 offsets
retain their original semantics.

Manifest code-point and UTF-8 byte ranges are zero-based, half-open ranges in
the original input. Result positions are not mixed into those fields. The
technical diff and content-preservation diff are distinct: byte-level or
canonical changes do not imply a semantic-equivalence claim.

The core has no clock, random source, locale, filesystem, or network input in
this pipeline. For equal text, profile, and TCS version, the result is
deterministic. Ambiguous observations such as ZWJ, ZWNJ, variation selectors,
combining marks, bidi controls, soft hyphen, Word Joiner, NBSP, and contextual
U+FEFF are preserved unless one narrowly defined profile rule explicitly
authorizes a transformation.

## `textscope-wasm`

The WASM crate depends on `textscope-core` and provides only interoperability
adaptation for browser consumers. It may translate inputs, outputs, and errors
at the boundary, but it must not own or duplicate domain rules.

The implemented browser boundary uses `wasm-bindgen` and exports ten
functions:

- `analyze_pdf_bytes(bytes)` receives a JavaScript `Uint8Array`, calls the
  bounded PDF forensic pipeline, and returns a tagged structured response
  without serializing source bytes.

- `analyze_docx_bytes(bytes)` receives a JavaScript `Uint8Array`, calls the
  complete core package/document pipeline, and returns a tagged structured
  response without serializing source bytes.
- `analyze_html_bytes(bytes)` receives a JavaScript `Uint8Array`, calls the
  complete core HTML pipeline, and returns a tagged structured response.
- `analyze_text_bytes(bytes, source_kind)` receives a JavaScript `Uint8Array`
  as a borrowed Rust byte slice, calls core ingestion, and returns a tagged
  success/error JSON response with decode metadata, source mappings, and scan.
- `scan_text(text)` calls the core scanner and returns a JSON string whose
  top-level fields are `characters`, `findings`, and `normalization`.
- `reveal_text(text)` calls the core reveal implementation and returns its
  string unchanged.
- `clean_text(text, profile)` parses a stable profile ID, calls the core clean
  pipeline, and returns the complete result as JSON.
- `available_clean_profiles()` returns profile metadata sourced from the core,
  including support state and active rule IDs.
- `tcs_version()` returns the core's formal TCS version.
- `compare_clipboard_representations(plain_text, html_text)` returns the
  core-owned deterministic comparison state for the two clipboard channels.

The JSON contract keeps the core's `snake_case` names. Finding kinds and
severities are stable `snake_case` strings. The JavaScript consumer performs
one explicit `JSON.parse` on each JSON output. On the 32-bit WASM target,
offset values fit exactly in JavaScript `Number`; the bridge does not convert
or recompute them.

Contract-only data transfer structs live in the bridge so the core remains
independent of Serde and browser interoperability. They copy fields and enum
labels but contain no Unicode classification, finding, normalization, reveal,
cleaning, diff, or verification rules. Controlled errors use fixed messages and
never echo the analyzed text.

Raw bytes are not Base64-encoded and are not serialized into a JSON byte array.
Only the structured analysis result is serialized. Encoding detection,
decoding, source mapping, and scanning are delegated to the core rather than
repeated in the bridge.

UI state, rendering behavior, analytics, network processing, and an independent
Unicode implementation do not belong in this layer. The bridge does not import
browser APIs and performs no logging or I/O.

## `web`

The TypeScript/React/Vite application collects Paste Text, explicitly reads Rich
Clipboard, or reads a selected local file and renders results obtained through
the WASM bridge.
`src/resource-limits.ts` owns the shared 10 MiB browser guard. `src/local-file.ts`
owns separate `.txt`/`.md`/`.markdown`, `.html`/`.htm`, and `.docx`/`.pdf`
allowlists and advisory browser metadata; it validates `File.size` before
`arrayBuffer()`. `src/wasm/client.ts` owns initialization and calls to generated
exports. `src/wasm/contract.ts` parses JSON, validates the shared required
structure, and provides TypeScript types. `src/wasm/docx-contract.ts` owns the
DOCX-only transfer types and structural validation; it contains no ZIP, XML,
OOXML, classification, or extraction rule.

React components do not call generated bindings directly. They may count
boolean flags already returned by Rust, but they do not classify Unicode,
construct findings, normalize text, or reproduce reveal placeholders.

Profile labels, descriptions, support state, and active rule identifiers are
loaded from Rust. The UI shows only supported profiles and requires an explicit
Clean action. It renders original and Clean Text separately using React text
nodes; cleaning never mutates the original textarea. Manifest and verification
fields are structural data returned through the contract rather than results
recomputed in TypeScript.

The native file picker is accessible and deliberately simple. TXT and Markdown
are decoded and displayed as literal text nodes; Markdown is not parsed or
rendered. React never receives permission to write the file. The UI displays
the base file name and relevant metadata but never requests or retains a full
local path. Replacing, removing, rejecting, or leaving a file clears stale
analysis and clean state.

HTML is selected in a dedicated mode. React receives only the structured Rust
result and inserts every user-controlled value through ordinary escaped text
children. Product code does not use `DOMParser`, `innerHTML`,
`dangerouslySetInnerHTML`, `iframe`, `document.write`, `eval`, `new Function`,
`fetch`, `XMLHttpRequest`, or `sendBeacon` for the supplied document. References
and inert script/style strings are display data only. Analyze, Reveal, and
Clean target `text_channels.document_text`; they never rewrite HTML source.

DOCX is also selected in a dedicated mode. React renders the Rust-produced
package summary, part/relationship evidence, observed metadata, findings,
comments, revisions, and separate text channels as escaped text. It never
unpacks ZIP, parses XML, follows a hyperlink, renders media, opens Word, or
rewrites the source. Analyze, Reveal, and Clean operate only on extracted body
text; Document Clean remains reserved and Clean produces text, never DOCX.

PDF is selected in a dedicated mode. `PdfInspector` renders only escaped Rust
summary, page text, metadata, fonts, actions, references, attachments,
forms/XFA, signatures, and findings. Analyze, Reveal, and Clean target the
extracted PDF text; they never render a PDF or mutate the uploaded bytes.

Rich Clipboard has its own inspector for advertised MIME representations,
plain text, inert HTML structural analysis, extracted text, deterministic
comparison, and provenance evidence. Analyze/Reveal/Clean use plain text when
present, otherwise extracted HTML text; both representations remain visible.

The PowerShell build pipeline compiles release WASM, remaps local build paths,
and runs the exactly matched `wasm-bindgen` CLI. Generated JavaScript,
declarations, and `.wasm` live under `apps/web/src/wasm/generated` and are
ignored because they are deterministic build artifacts. Vite includes them in
the local development and static production bundles.

Raw file bytes, decoded text, analyzed text, and clean results must not be sent
to remote services. The UI must not introduce upload, telemetry, or analytics.
Vite development asset requests and HMR remain local development infrastructure
and are not text-processing endpoints.

## Future consumers

Because the core is platform-independent, a future CLI or Tauri adapter can
depend directly on it. Those adapters can provide their own presentation and
I/O boundaries without copying Unicode or canonicalization rules. Neither is
part of the current scope.
