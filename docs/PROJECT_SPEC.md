# TextScope project specification

## Product objective

TextScope is a local-first tool for forensic inspection and deterministic,
content-preserving canonicalization of pasted or file-sourced text. Its purpose
is to reveal what text contains, classify relevant observations, apply an
explicitly defined cleaning policy, and verify the result without sending the
text elsewhere.

The implemented input scope is Paste Text, Rich Clipboard, literal TXT and
Markdown files, and inert forensic analysis of local HTML, DOCX, and PDF files.

## Conceptual flow

```text
PASTE TEXT / RICH CLIPBOARD → UNICODE TEXT ──────────┐
                                                    ├→ SCAN
TXT / MARKDOWN → RAW BYTES → STRICT DECODE ─────────┘
HTML → RAW BYTES → HTML ENCODING EVIDENCE → STRICT DECODE
     → HTML5 STRUCTURE → EXTRACTED DOCUMENT TEXT ────┘
DOCX / PDF → RAW BYTES → BOUNDED FORENSICS → EXTRACTED DOCUMENT TEXT ────┘
→ REVEAL
→ CLASSIFY
→ CLEAN
→ RESCAN
→ VERIFY
→ CLEAN TEXT
```

## Inputs and strict file ingestion

The supported input modes are:

- **Paste Text:** Unicode text supplied by the browser textarea.
- **Rich Clipboard:** an explicit user-triggered snapshot of advertised
  `text/plain` and `text/html` representations. Both are retained; plain text
  uses strict ingestion and HTML uses the inert HTML adapter. Unsupported MIME
  types are listed without reading their bytes. A deterministic comparison
  reports `equivalent`, `technically_different`, `content_different`, or
  `needs_review`. Clipboard access never polls, persists, logs, writes, or
  contacts a network and is bounded to 10 MiB before WASM.
- **TXT:** local files with the `.txt` extension.
- **Markdown:** local files with `.md` or `.markdown`; syntax remains literal
  text and is never rendered as HTML.
- **HTML:** local files with `.html` or `.htm`; the Rust core parses them as
  untrusted data and returns structure, observed metadata, references,
  comments, findings, and explicitly extracted text. The browser never renders
  or executes the supplied document.
- **DOCX:** local files with `.docx` only; the Rust core treats the bytes as an
  untrusted ZIP/OOXML package and returns inert package, relationship,
  metadata, document-structure, provenance, and extracted-text observations.
  `.docm`, `.dotm`, `.doc`, `.rtf`, `.odt`, and renamed non-DOCX ZIPs are not
  silently accepted.
- **PDF:** local files with `.pdf` only; the Rust core treats the bytes as
  untrusted binary evidence and returns bounded object, page, xref/revision,
  metadata, resource, action, reference, attachment, form, signature, and
  extracted-text observations. PDF bytes are never rendered or rewritten.

The browser checks a single 10 MiB UI limit before calling `File.arrayBuffer()`
and sends the resulting `Uint8Array` directly to WASM. The Rust core accepts a
byte slice and strictly supports UTF-8 without BOM, UTF-8 with BOM, UTF-16 LE
with BOM, and UTF-16 BE with BOM. Empty bytes are valid empty UTF-8 text.

No lossy decoding, replacement with U+FFFD, charset guessing, locale, file
extension, MIME, or operating-system code page participates in the encoding
decision. A strong alternating-NUL pattern resembling BOM-less UTF-16 is
reported as ambiguous and is not decoded.

For every decoded Unicode scalar, the core records a zero-based half-open range
in the original source bytes alongside the decoded UTF-8 byte offset, UTF-16
code-unit offset, and code-point index. These coordinate systems remain
separate. A file BOM is encoding metadata and does not become an artificial
textual U+FEFF. Ingestion preserves line endings; TCS rules can transform them
only after an explicit Clean action.

The UI retains no full local path and treats the browser MIME as advisory. It
does not upload, write, overwrite, export, or save the source file.

### HTML encoding and structural analysis

HTML has its own adapter and does not alter the TXT/Markdown policy. The HTML
path first applies the same strict byte decoder, then tokenizes the decoded
source to inspect `meta charset` and historical `http-equiv=Content-Type`
declarations whose source start lies in the first 1024 raw bytes. Supported
BOM-less declarations must identify UTF-8. A declaration for a legacy or other
unsupported charset produces `UnsupportedDeclaredEncoding`; there is no
Windows-1252 fallback or statistical sniffing.

A recognized BOM has higher precedence than every meta declaration. Parsing
continues with the BOM-selected UTF-8/UTF-16 encoding and any disagreement is
retained as encoding evidence plus an `encoding_declaration_conflict` finding.
No declaration is treated as permission for lossy decoding.

The core uses an HTML5 tree builder for tolerant structure and a separate
WHATWG tokenizer for source-observed start tags, attributes, comments,
doctype, text representations, and spans. Parsed nodes receive a raw range only
when they correlate unambiguously with contiguous source tokens. Otherwise the
range is explicitly unavailable; reconstructed offsets are never presented as
source evidence.

Text nodes are classified into visible-content candidate, title, metadata,
script, style, template, and noscript channels. Only visible document text
enters the default Unicode scan, Reveal, and explicit TCS Clean operations.
Comments, script, CSS, metadata, templates, and noscript content remain outside
that main channel. HTML character references are decoded by the parser, so for
example `&nbsp;` reaches the scanner as U+00A0 while its source spelling remains
available when the token mapping is reliable.

Source text-node whitespace is preserved. When adjacent visible text nodes
belong to different recognized block containers, or are separated by `br`, the
extractor inserts one LF only if neither boundary already contains whitespace.
This deterministic representation is extracted document text, not a byte copy
or rewritten version of the HTML.

HTML analysis never follows `href`, `src`, `srcset`, `action`, `poster`, `data`,
or `formaction`; it only records and classifies their declared values. Script
elements, recognized inline event-handler attributes, and conservative hidden
markers (`hidden`, `aria-hidden=true`, exact inline `display:none`, or exact
inline `visibility:hidden`) are inert structural findings, not claims about
intent or rendered state.

The HTML core rejects rather than truncates input beyond these explicit bounds:
10 MiB source bytes; 300,000 tokenizer or parsed-node records; 100,000 source
start tags; 500,000 attributes; 512 parsed-tree ancestors; 10,000 recoveries;
50,000 comments; 1 MiB per attribute value or comment; and 2 MiB per text
token. These are resource-safety limits, not assertions that a document is
harmful or corrupt.

### DOCX package and document analysis

DOCX input follows a separate, immutable evidence pipeline:

```text
original DOCX bytes
→ bounded in-memory ZIP inspection
→ OPC content types and relationships
→ WordprocessingML structure and observed metadata
→ explicit text channels with provenance
→ body Unicode scan / Reveal / explicit TCS Clean / independent rescan
```

The minimum valid package has `[Content_Types].xml`, `_rels/.rels`, and
`word/document.xml`, with a standard WordprocessingML main content type and a
root `officeDocument` relationship to that part. The result classifies a
successful package as `valid_wordprocessing_package`; controlled errors cover
invalid ZIP, unsupported package types, missing required parts, duplicate
names, encryption, unsupported compression, malformed required XML or
relationships, and resource bounds. ZIP64 is recorded when observable.

No archive member is extracted to the filesystem. Entry names are evidence:
NUL, absolute/drive/UNC paths, `..` traversal, backslash ambiguity, non-UTF-8
raw names, repeated separators, `.` components, and Unicode-normalization
ambiguity are never silently normalized into safe names. External
relationships, media, embedded objects, ActiveX/VBA-related parts, custom XML,
and unclassified binary parts are inventoried or reported but never opened,
followed, rendered, fetched, or executed.

The browser rejects more than 10 MiB before `File.arrayBuffer()` and the core
independently rejects more than 10 MiB. Core limits are 2,048 entries; 64 MiB
aggregate uncompressed bytes; 16 MiB per part; 8 MiB per parsed XML part;
200:1 per-entry or aggregate expansion; 1,024 bytes per entry name; 20,000
relationships; 200,000 text segments; 250,000 XML nodes per part; and XML
ancestor depth 256. These bounds allow ordinary document packages while
placing a deterministic ceiling on browser memory, decompression, XML-tree,
relationship, and result growth. A limit causes complete controlled rejection,
never partial silent output.

WordprocessingML extraction concatenates runs inside paragraphs, separates
paragraphs with LF, maps `w:tab` to TAB, `w:br`/`w:cr` to LF,
`w:noBreakHyphen` to U+2011, and `w:softHyphen` to U+00AD. It does not trim
`w:t`; `xml:space="preserve"` is retained as explicit evidence. Table cells
use TAB and rows use LF, including deterministic empty cells. This is
**Extracted Document Text**, not a pixel layout or byte copy of Word output.

Body, headers, footers, footnotes, endnotes, comments, and deleted content are
separate channels. Internal separator footnotes/endnotes (non-positive IDs)
are omitted. Inserted and move-to text participates in the current body
channel; deleted and move-from text remains in revision/deleted channels.
TextScope never accepts or rejects a revision. Explicit `w:vanish` is reported
as an explicit hidden-run marker without claiming effective rendered
visibility, and its text remains preserved.

Core, app, custom-property, and comment values are **observed stored
declarations**, not verified attribution. XML source ranges are emitted only
when the strict decoder proves their decoded and uncompressed-part mapping.
Compressed-package offsets for XML nodes remain unavailable because the
mapping cannot be proven. The original DOCX bytes are never rewritten.

Document Clean remains reserved. Existing TCS 0.1.0 profiles may be applied
explicitly to the extracted body text and produce separate Clean Text,
manifest, diff, and verification artifacts. None of those operations changes
package bytes, metadata, comments, relationships, hidden content, or revision
state.

### PDF forensics and PDF adapter

PDF input follows an immutable binary-evidence pipeline: the browser passes a
`.pdf` byte snapshot to WASM, and the Rust Core performs bounded `lopdf`
inspection of header/version, EOF/startxref, xref/revision and encryption
evidence, objects/streams/page tree/resources/fonts/images, Info/XMP metadata,
annotations/actions/URI references, embedded-file declarations, forms/XFA,
signature dictionaries, and content-stream text.

The browser and Core reject more than 10 MiB. Core limits include 100,000
objects, 10,000 pages, 8 MiB decoded and individual streams, 200:1
decoded-to-encoded ratio where measurable, 2 MiB font mapping streams,
reference depth 256, 200,000 content operations and text segments, 50,000
annotations/actions, 10,000 embedded files, 1 MiB metadata, and 50,000
findings. A limit is a complete controlled rejection, never silent partial
output.

The adapter never renders a PDF, follows a URI, executes JavaScript or another
action, opens or extracts an attachment, invokes OCR, validates cryptographic
signatures, or rewrites source bytes. Active content is structural evidence
only. Text extraction uses declared font encodings and ToUnicode mappings when
available; unavailable mappings are reported honestly rather than replaced
with fabricated characters. Page text is parser-order evidence, not a claim
about visual reading order. A PDF with no text operators is reported as having
no text layer. Stored metadata, references, signatures, and encryption state
are observations, not attribution, intent, trust, or validity claims.

## Architecture

The implemented architecture is:

```text
Rust Core → thin WASM bridge → TypeScript / React / Vite
```

The Rust core is the source of truth for scanning, Unicode classification,
canonicalization, diffing, and verification. The WASM bridge adapts core
capabilities for browser use. The frontend presents inputs and results without
reimplementing domain rules.

## Product principles

- Local-first and privacy-preserving: analyzed text remains on the device.
- Deterministic and reproducible: identical inputs and policy versions produce
  identical results.
- Testable and auditable: rules and transformations require explicit evidence.
- Secure: no hidden network processing, analytics, or telemetry.
- Content-preserving: meaningful content is retained unless an explicit,
  versioned rule authorizes a transformation.
- Minimal-dependency: external code is introduced only after review.

## Observation, detection, and attribution

- **Observation** records what is present in the text, without judging intent.
- **Detection** applies a defined rule to an observation or sequence.
- **Attribution** claims a source, purpose, or intent. TextScope must not infer
  attribution merely from the presence of a code point or pattern.

Rich Clipboard and document adapters expose a common `ProvenanceEvidence`
record with `EvidenceKind`, `EvidenceSource`, `EvidenceLevel`, and
`VerificationState`. The levels distinguish Observation, Claim, Verification,
and Attribution; statuses distinguish Observed, Declared, Verified, Not
Verified, Unsupported, and Conflict. Stored HTML generator, DOCX creator, and
PDF author/producer fields remain declarations. Attribution is Unsupported
unless independent evidence is available.

Invisible does not mean removable. Visibility, semantics, context, and the
active cleaning standard must be considered separately.

Examples include:

- Zero Width Joiner (ZWJ) may form emoji sequences or orthographic joins.
- Zero Width Non-Joiner (ZWNJ) may carry legitimate joining semantics in a
  writing system.
- Variation selectors choose a presentation variant and can be meaningful.
- Combining marks form grapheme content with neighboring characters; removing
  them can corrupt text.
- Bidirectional controls affect display order and may be legitimate or may need
  contextual review. Their presence alone does not establish malicious intent.

## TextScope Clean Standard 0.1.0

TCS 0.1.0 is the first pre-1.0 cleaning contract for pasted text. The version,
rule metadata, profile membership, transformations, and verification behavior
are defined in `textscope-core` rather than in the browser layers.

Phases 3 and 4 pass strictly decoded or extracted document text into this same
contract. Phase 5 does the same for DOCX body text. File encoding, HTML/OOXML
parsing, and extraction are ingestion concerns, so no TCS rule or version
changed.

The implemented registry contains four rules:

| Rule ID | Purpose |
| --- | --- |
| `TCS-NORMALIZATION-001` | Represent canonical Unicode equivalents in NFC. |
| `TCS-LINE-ENDINGS-001` | Represent CRLF and lone CR line breaks as LF. |
| `TCS-CONTROL-001` | Remove exactly one leading U+FEFF only when it is an unambiguous pasted-text marker. |
| `TCS-WHITESPACE-001` | Replace NBSP with U+0020 only under Pure Plain Text and require content review. |

Each rule has a stable ID, version, title, description, category, default
policy, and operation. Every applied transformation records the rule and its
original half-open code-point and UTF-8 byte ranges.

## Pasted-text clean profiles

- **Safe Clean:** uses only line-ending canonicalization. Ambiguous Unicode is
  preserved and reported.
- **Canonical Clean:** uses NFC, LF line endings, and the unambiguous leading
  U+FEFF rule.
- **Pure Plain Text:** uses all Canonical rules plus explicit NBSP-to-space
  replacement. Because that changes non-breaking layout behavior, content
  preservation and verification report `needs_review`.
- **Document Clean:** remains reserved. HTML now supplies document context, but
  TCS 0.1.0 has no normative rule set that would distinguish this profile from
  an existing supported profile; activating only a new name would not justify
  a standard change.
- **Developer Clean:** reserved until source-code-specific preservation rules
  are specified.

All supported profiles are deterministic and idempotent. Reapplying the same
profile to Clean Text produces no additional transformations.

## Current cleaning outputs

Every successful clean operation returns:

- **Clean Text:** a separate result; the original input is retained unchanged.
- **Transformation Manifest:** a complete ordered list of declared changes,
  including rule, operation, original ranges, replacement, reason, before, and
  after.
- **Technical Diff:** deterministic byte/code-point counts and transformation
  count.
- **Content Preservation Diff:** a separate conservative assessment of whether
  the implemented technical rule can claim preservation.
- **Verification Report:** original and clean scan summaries, applied changes,
  remaining findings, preservation status, overall status, and explicit
  invariant checks.

Verification performs a new call to the real scanner on Clean Text. It checks
manifest replay, profile rule membership, absence of undeclared changes,
rescan, re-evaluation of expected findings, visibility of preserved findings,
deterministic repetition, and idempotent reapplication.

The overall statuses are `verified`, `verified_with_remaining_findings`,
`needs_review`, and `failed`. Remaining findings are not presented as a
failure or silently removed; they remain available for inspection.

## Current phases

- **Phase 0 — Foundation (complete):** repository structure, architectural
  boundaries, persistent documentation, and quality gates.
- **Phase 1A — Basic Unicode Core Scanner (complete):** deterministic scalar
  records, independent UTF-8/UTF-16/code-point offsets, conservative findings,
  normalization inspection, and technical reveal representation.
- **Phase 1B — WASM Bridge (complete):** explicit JSON boundary over the core.
- **Phase 1C — Web Integration (complete):** local React UI and real Edge gate.
- **Phase 2 — TCS Clean Engine for pasted text (complete):** versioned policy,
  manifest, diffs, independent rescan, verification, WASM contract, and web UI.
- **Phase 3 — Byte/encoding foundation and text file adapters (complete):**
  strict UTF-8/UTF-16 decoding, BOM policy, source-byte provenance, literal TXT
  and Markdown adapters, byte-safe WASM boundary, native file UI, and Edge E2E.
- **Phase 4 — HTML forensics and HTML adapter (complete):** strict HTML byte
  ingestion, BOM/meta evidence, tolerant HTML5 parsing, source-token spans,
  structural and metadata inspection, inert reference/findings reporting,
  explicit text channels, extracted-text Scan/Reveal/Clean, and Edge safety
  validation.
- **Phase 5 — DOCX forensics and document adapter (complete):** bounded
  in-memory ZIP/OOXML inspection, package validation, safe part inventory,
  content types, inert relationships, observed metadata, document structure,
  separate text/revision/comment channels, honest provenance, Unicode/TCS
  integration, and real Edge privacy validation.
- **Phase 6 — PDF forensics and PDF adapter (complete):** bounded immutable
  binary parsing, object/page/xref/resource inventory, metadata and active
  content structure, honest text/ToUnicode status, extracted-text pipeline,
  WASM contract, local UI, malformed/resource-limit tests, and real Edge
  privacy validation.
- **Phase 7 — Product hardening and provenance (complete):** explicit Rich
  Clipboard read and paste fallback, preserved plain/HTML/advertised MIME
  representations, deterministic comparison, unified provenance evidence,
  C2PA evaluation, accessibility/responsive checks, supply-chain/static audits,
  cross-format invariants, and release-readiness documentation.

## Explicitly outside the current scope

- Backend services, databases, accounts, authentication, or cloud processing
- Analytics, telemetry, AI APIs, or remote processing of analyzed text
- CLI, Tauri application, legacy/rich document adapters, and batch processing
- HTML preview/rendering, source rewriting, sanitization output, JavaScript or
  CSS execution, crawling, and external resource resolution
- Background or automatic clipboard access (Rich Clipboard remains explicit)
- File streaming, drag-and-drop, export, save, or source-file overwrite
- Charset heuristics and legacy encoding detection
- Document Clean and Developer Clean behavior
- Semantic rewriting, grammar or spelling correction, paraphrasing,
  transliteration, authorship detection, and AI detection
