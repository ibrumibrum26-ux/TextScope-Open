import type { LocalPdfFileMetadata } from './local-file.ts'
import type {
  PdfActionRecord,
  PdfAnalysis,
  PdfAnnotationRecord,
  PdfEmbeddedFileRecord,
  PdfExternalReference,
  PdfFinding,
  PdfFontRecord,
  PdfImageRecord,
  PdfMetadataEntry,
  PdfPageSummary,
  PdfSignatureRecord,
} from './wasm/pdf-contract.ts'
import { ProvenanceEvidenceList } from './ProvenanceEvidenceList.tsx'

function displayText(value: string): string {
  return value === '' ? '(no extracted text)' : value
}

function sourceLabel(source: { object_number: number | null; raw_byte_offset: number | null; context: string }): string {
  const object = source.object_number === null ? 'direct object' : `object ${source.object_number}`
  const offset = source.raw_byte_offset === null ? 'offset unavailable' : `byte ${source.raw_byte_offset}`
  return `${object}; ${offset}; ${source.context}`
}

function MetadataList({ entries }: { entries: PdfMetadataEntry[] }) {
  if (entries.length === 0) return <p>No Info metadata declarations were observed.</p>
  const labels: Record<string, string> = {
    Title: 'Declared Title', Author: 'Declared Author', Subject: 'Declared Subject',
    Keywords: 'Declared Keywords', Creator: 'Declared Creator', Producer: 'Declared Producer',
    CreationDate: 'Declared Creation Date', ModDate: 'Declared Modification Date', Trapped: 'Declared Trapped',
  }
  return <dl className="compact-list">{entries.map((entry, index) => (
    <div key={`${entry.key}-${index}`}><dt>{labels[entry.key] ?? `Declared ${entry.key}`}</dt><dd>{entry.value}</dd></div>
  ))}</dl>
}

function ActionList({ actions }: { actions: PdfActionRecord[] }) {
  if (actions.length === 0) return <p>No action dictionaries were observed.</p>
  return <ol>{actions.map((action) => (
    <li key={action.sequence}>
      <div className="finding-heading"><code>{action.kind}</code><span className="severity">structural only</span></div>
      <p>{action.target ?? 'Target not declared'}</p>
      <p className="result-note">{sourceLabel(action.source)}</p>
    </li>
  ))}</ol>
}

function AnnotationList({ annotations }: { annotations: PdfAnnotationRecord[] }) {
  if (annotations.length === 0) return <p>No annotations were observed.</p>
  return <ol>{annotations.map((annotation) => (
    <li key={annotation.sequence}>
      <code>page {annotation.page_index + 1} · {annotation.subtype}</code>
      <p>{annotation.contents ?? 'No annotation contents declared.'}</p>
      {annotation.uri !== null && <p>{annotation.uri}</p>}
    </li>
  ))}</ol>
}

function ExternalReferenceList({ references }: { references: PdfExternalReference[] }) {
  if (references.length === 0) return <p>No external references were observed.</p>
  return <ol>{references.map((reference) => (
    <li key={reference.sequence}><code>{reference.kind}</code> {reference.value}</li>
  ))}</ol>
}

function EmbeddedFileList({ files }: { files: PdfEmbeddedFileRecord[] }) {
  if (files.length === 0) return <p>No embedded-file declarations were observed.</p>
  return <ol>{files.map((file) => (
    <li key={file.sequence}>
      <code>{file.declared_filename ?? 'unnamed file'}</code>
      {file.mime_type !== null && <span> · {file.mime_type}</span>}
      {file.size !== null && <span> · {file.size} bytes</span>}
    </li>
  ))}</ol>
}

function FontList({ fonts }: { fonts: PdfFontRecord[] }) {
  if (fonts.length === 0) return <p>No page font resources were observed.</p>
  return <ol>{fonts.map((font) => (
    <li key={font.sequence}>
      <code>{font.resource_name}</code> · {font.base_font ?? 'unnamed font'} · mapping {font.mapping_status}
      <p className="result-note">ToUnicode: {font.to_unicode_present ? 'present' : 'not declared'}; {sourceLabel(font.source)}</p>
    </li>
  ))}</ol>
}

function ImageList({ images }: { images: PdfImageRecord[] }) {
  if (images.length === 0) return <p>No image XObjects were observed.</p>
  return <ol>{images.map((image) => (
    <li key={image.sequence}>
      <code>object {image.object.object_number ?? 'direct'}</code> · pages {image.page_indices.map((page) => page + 1).join(', ')}
      <p className="result-note">
        {image.width ?? '?'}×{image.height ?? '?'} · filter {image.filter ?? 'not declared'} · color space {image.color_space ?? 'not declared'}
      </p>
    </li>
  ))}</ol>
}

function SignatureList({ signatures }: { signatures: PdfSignatureRecord[] }) {
  if (signatures.length === 0) return <p>No signature structures were observed.</p>
  return <ol>{signatures.map((signature) => (
    <li key={signature.sequence}>
      <code>{signature.sub_filter ?? 'signature dictionary'}</code>
      <p>ByteRange: {signature.byte_range_present ? 'present' : 'not declared'} · Contents: {signature.contents_present ? 'present' : 'not declared'}</p>
    </li>
  ))}</ol>
}

function PageList({ pages }: { pages: PdfPageSummary[] }) {
  if (pages.length === 0) return <p>No page nodes were observed.</p>
  return <ol>{pages.map((page) => (
    <li key={page.page_index}>
      <code>Page {page.page_index + 1}</code> · {page.content_stream_count} content stream(s) · {page.annotation_count} annotation(s)
      {page.text !== '' && <pre className="decoded-output">{page.text}</pre>}
    </li>
  ))}</ol>
}

function FindingList({ findings }: { findings: PdfFinding[] }) {
  if (findings.length === 0) return <p>No PDF structural findings.</p>
  return <ol className="findings-list">{findings.map((finding) => (
    <li key={finding.sequence}>
      <div className="finding-heading"><code>{finding.kind}</code><span className="severity">{finding.severity}</span></div>
      <p>{finding.description}</p>
    </li>
  ))}</ol>
}

export function PdfInspector({ metadata, analysis }: { metadata: LocalPdfFileMetadata; analysis: PdfAnalysis }) {
  const version = analysis.summary.version?.declared ?? 'Not declared'
  const xmp = analysis.metadata.xmp

  return (
    <section aria-labelledby="pdf-source-heading" data-testid="pdf-source-panel">
      <div className="section-heading-row">
        <div>
          <p className="eyebrow dark">In-memory Rust PDF forensics</p>
          <h2 id="pdf-source-heading">Local PDF source</h2>
        </div>
        <p className="source-unchanged" data-testid="source-unchanged">Source file unchanged</p>
      </div>

      <dl className="source-metadata">
        <div><dt>File</dt><dd data-testid="file-name">{metadata.file_name}</dd></div>
        <div><dt>Byte size</dt><dd data-testid="raw-byte-length">{analysis.summary.raw_byte_length}</dd></div>
        <div><dt>PDF version</dt><dd data-testid="pdf-version">{version}</dd></div>
        <div><dt>Pages</dt><dd data-testid="pdf-pages">{analysis.summary.page_count}</dd></div>
        <div><dt>Objects / streams</dt><dd>{analysis.summary.object_count} / {analysis.summary.stream_count}</dd></div>
        <div><dt>XRef</dt><dd>{analysis.summary.xref_kind} · {analysis.summary.startxref_count} startxref</dd></div>
        <div><dt>Revisions</dt><dd>{analysis.revisions.length}</dd></div>
        <div><dt>Encryption</dt><dd data-testid="pdf-encryption">{analysis.summary.encryption}</dd></div>
        <div><dt>Fonts / images</dt><dd>{analysis.summary.font_count} / {analysis.summary.image_count}</dd></div>
        <div><dt>Annotations / actions</dt><dd>{analysis.summary.annotation_count} / {analysis.summary.action_count}</dd></div>
        <div><dt>External refs</dt><dd>{analysis.external_references.length}</dd></div>
        <div><dt>Embedded files</dt><dd>{analysis.summary.embedded_file_count}</dd></div>
        <div><dt>Forms / XFA</dt><dd>{analysis.forms.present ? 'Present' : 'Absent'} / {analysis.forms.xfa_present ? 'Present' : 'Absent'}</dd></div>
        <div><dt>Signatures</dt><dd>{analysis.signatures.length}</dd></div>
        <div><dt>Text extraction</dt><dd data-testid="pdf-text-status">{analysis.text_extraction_status}</dd></div>
      </dl>

      <p className="safety-note">
        The Rust Core treated the PDF as untrusted binary bytes. It did not render, execute actions or JavaScript,
        fetch URI targets, open attachments, invoke OCR, or rewrite the source. Active content is structural evidence only.
      </p>

      <section aria-labelledby="pdf-extracted-text-heading">
        <h3 id="pdf-extracted-text-heading">Extracted PDF Text</h3>
        <p className="result-note">Text is reconstructed from content streams in parser order; visual reading order is not guaranteed. OCR is not implemented.</p>
        <pre className="decoded-output" data-testid="pdf-extracted-text">{displayText(analysis.document_text)}</pre>
      </section>

      <div className="inspector-grid">
        <section aria-labelledby="pdf-pages-heading"><h3 id="pdf-pages-heading">Pages</h3><PageList pages={analysis.pages} /></section>
        <section aria-labelledby="pdf-metadata-heading" data-testid="pdf-metadata"><h3 id="pdf-metadata-heading">Observed PDF Metadata</h3><p className="result-note">Stored declarations only; they are not verified attribution.</p><MetadataList entries={analysis.metadata.info_entries} />{xmp !== null && <p className="result-note">XMP: {xmp.well_formed_xml ? 'well-formed XML' : 'malformed or unavailable'} ({xmp.properties.length} properties)</p>}</section>
        <section aria-labelledby="pdf-fonts-heading"><h3 id="pdf-fonts-heading">Fonts and ToUnicode</h3><FontList fonts={analysis.fonts} /></section>
        <section aria-labelledby="pdf-images-heading"><h3 id="pdf-images-heading">Image XObjects</h3><ImageList images={analysis.images} /></section>
        <section aria-labelledby="pdf-annotations-heading"><h3 id="pdf-annotations-heading">Annotations</h3><AnnotationList annotations={analysis.annotations} /></section>
        <section aria-labelledby="pdf-actions-heading"><h3 id="pdf-actions-heading">Actions and JavaScript</h3><ActionList actions={analysis.actions} /></section>
        <section aria-labelledby="pdf-external-heading"><h3 id="pdf-external-heading">External references</h3><ExternalReferenceList references={analysis.external_references} /></section>
        <section aria-labelledby="pdf-embedded-heading"><h3 id="pdf-embedded-heading">Embedded files</h3><EmbeddedFileList files={analysis.embedded_files} /></section>
        <section aria-labelledby="pdf-signatures-heading"><h3 id="pdf-signatures-heading">Signature structures</h3><SignatureList signatures={analysis.signatures} /></section>
        <section aria-labelledby="pdf-findings-heading"><h3 id="pdf-findings-heading">PDF findings</h3><FindingList findings={analysis.findings} /></section>
      </div>

      <ProvenanceEvidenceList evidence={analysis.provenance} testId="pdf-provenance" />

      <p className="result-note">Source MIME type is advisory only. TextScope keeps the original local file unchanged and does not expose local paths.</p>
    </section>
  )
}
