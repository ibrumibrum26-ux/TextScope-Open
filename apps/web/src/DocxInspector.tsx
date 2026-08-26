import type { LocalDocxFileMetadata } from './local-file.ts'
import type {
  DocxAnalysis,
  DocxMetadataProperty,
  DocxPartTextChannel,
} from './wasm/docx-contract.ts'
import { ProvenanceEvidenceList } from './ProvenanceEvidenceList.tsx'

function displayText(value: string): string {
  return value === '' ? '(empty text)' : value
}

function metadataLabel(property: DocxMetadataProperty): string {
  const labels: Record<string, string> = {
    creator: 'Declared creator',
    lastModifiedBy: 'Declared last modified by',
    created: 'Declared creation timestamp',
    modified: 'Declared modification timestamp',
  }
  return labels[property.name] ?? `Declared ${property.name}`
}

function TextChannel({ title, channels }: { title: string; channels: DocxPartTextChannel[] }) {
  if (channels.length === 0) return null

  return (
    <section aria-label={title}>
      <h3>{title}</h3>
      {channels.map((channel) => (
        <div key={channel.part_name}>
          <p className="result-note"><code>{channel.part_name}</code></p>
          <pre className="decoded-output">{displayText(channel.text)}</pre>
        </div>
      ))}
    </section>
  )
}

export function DocxInspector({
  metadata,
  analysis,
}: {
  metadata: LocalDocxFileMetadata
  analysis: DocxAnalysis
}) {
  const metadataProperties = [
    ...analysis.metadata.core_properties,
    ...analysis.metadata.app_properties,
    ...analysis.metadata.custom_properties,
  ]
  const inertParts = analysis.parts.filter((part) =>
    part.classification === 'media' ||
    part.classification === 'embedded_object' ||
    part.classification === 'active_content_related' ||
    part.classification === 'custom_xml',
  )

  return (
    <section aria-labelledby="docx-source-heading" data-testid="docx-source-panel">
      <div className="section-heading-row">
        <div>
          <p className="eyebrow dark">In-memory Rust ZIP/OOXML forensics</p>
          <h2 id="docx-source-heading">Local DOCX source</h2>
        </div>
        <p className="source-unchanged" data-testid="source-unchanged">Source file unchanged</p>
      </div>

      <dl className="source-metadata">
        <div><dt>File</dt><dd data-testid="file-name">{metadata.file_name}</dd></div>
        <div><dt>Byte size</dt><dd data-testid="raw-byte-length">{analysis.package.raw_byte_length}</dd></div>
        <div><dt>Package status</dt><dd data-testid="docx-package-status">{analysis.package.status}</dd></div>
        <div><dt>Parts</dt><dd data-testid="docx-parts">{analysis.package.entry_count}</dd></div>
        <div><dt>Main document</dt><dd data-testid="docx-main-document">{analysis.summary.main_document_present ? 'Present' : 'Absent'}</dd></div>
        <div><dt>Paragraphs</dt><dd data-testid="docx-paragraphs">{analysis.summary.paragraphs}</dd></div>
        <div><dt>Runs</dt><dd data-testid="docx-runs">{analysis.summary.runs}</dd></div>
        <div><dt>Tables</dt><dd data-testid="docx-tables">{analysis.summary.tables}</dd></div>
        <div><dt>Headers</dt><dd data-testid="docx-headers">{analysis.summary.headers}</dd></div>
        <div><dt>Footers</dt><dd data-testid="docx-footers">{analysis.summary.footers}</dd></div>
        <div><dt>Footnotes</dt><dd data-testid="docx-footnotes">{analysis.summary.footnotes}</dd></div>
        <div><dt>Endnotes</dt><dd data-testid="docx-endnotes">{analysis.summary.endnotes}</dd></div>
        <div><dt>Comments</dt><dd data-testid="docx-comments-count">{analysis.summary.comments}</dd></div>
        <div><dt>Tracked changes</dt><dd data-testid="docx-revisions-count">{analysis.summary.tracked_changes}</dd></div>
        <div><dt>Explicit hidden runs</dt><dd data-testid="docx-hidden-count">{analysis.summary.explicit_hidden_runs}</dd></div>
        <div><dt>External relationships</dt><dd data-testid="docx-external-count">{analysis.summary.external_relationships}</dd></div>
        <div><dt>Media</dt><dd data-testid="docx-media-count">{analysis.summary.media_parts}</dd></div>
        <div><dt>Embedded objects</dt><dd data-testid="docx-embedded-count">{analysis.summary.embedded_object_parts}</dd></div>
        <div><dt>Compressed bytes</dt><dd>{analysis.package.compressed_total}</dd></div>
        <div><dt>Uncompressed bytes</dt><dd>{analysis.package.uncompressed_total}</dd></div>
        <div><dt>Compression ratio</dt><dd>{(analysis.package.compression_ratio_milli / 1000).toFixed(3)}:1</dd></div>
        <div><dt>ZIP64 observed</dt><dd>{analysis.package.zip64_present ? 'Yes' : 'No'}</dd></div>
      </dl>

      <p className="safety-note">
        The Rust Core treated this DOCX as untrusted ZIP/OOXML data. It did not execute macros,
        embedded objects, Office automation, or relationships; it did not extract files to disk.
      </p>

      <section aria-labelledby="docx-extracted-text-heading">
        <h3 id="docx-extracted-text-heading">Extracted Document Text</h3>
        <p className="result-note">
          This is a textual representation derived from the OOXML structure, not a byte-identical
          copy of the source file.
        </p>
        <pre className="decoded-output" data-testid="extracted-document-text">
          {displayText(analysis.text_channels.document_text)}
        </pre>
      </section>

      <div className="inspector-grid">
        <section aria-labelledby="docx-metadata-heading" data-testid="docx-metadata">
          <h3 id="docx-metadata-heading">Observed DOCX Metadata</h3>
          <p>Stored declarations only; they are not verified attribution.</p>
          {metadataProperties.length === 0 ? <p>No metadata declarations were observed.</p> : (
            <dl className="compact-list">
              {metadataProperties.map((property) => (
                <div key={`${property.source_location.part_name}-${property.sequence}`}>
                  <dt>{metadataLabel(property)}</dt>
                  <dd>{property.value}</dd>
                </div>
              ))}
            </dl>
          )}
        </section>

        <section aria-labelledby="docx-relationships-heading" data-testid="docx-relationships">
          <h3 id="docx-relationships-heading">Relationships</h3>
          {analysis.relationships.length === 0 ? <p>No relationships were observed.</p> : (
            <ol>
              {analysis.relationships.map((relationship) => (
                <li key={relationship.sequence}>
                  <code>{relationship.id}</code>{' '}
                  <span className="severity">{relationship.external ? 'external' : 'internal'}</span>
                  <p>{relationship.relationship_type}</p>
                  <p>{relationship.target}</p>
                </li>
              ))}
            </ol>
          )}
        </section>

        <section aria-labelledby="docx-comments-heading" data-testid="docx-comments">
          <h3 id="docx-comments-heading">Comments</h3>
          {analysis.comments.length === 0 ? <p>No comments were observed.</p> : (
            <ol>
              {analysis.comments.map((comment) => (
                <li key={comment.sequence}>
                  <p>Declared author: {comment.declared_author ?? 'Not declared'}</p>
                  <pre>{displayText(comment.text)}</pre>
                </li>
              ))}
            </ol>
          )}
        </section>

        <section aria-labelledby="docx-revisions-heading" data-testid="docx-revisions">
          <h3 id="docx-revisions-heading">Tracked / Deleted Content</h3>
          <p>Observed only; TextScope did not accept or reject any change.</p>
          {analysis.revisions.length === 0 ? <p>No tracked changes were observed.</p> : (
            <ol>
              {analysis.revisions.map((revision) => (
                <li key={revision.sequence}>
                  <span className="severity">{revision.kind}</span>
                  <pre>{displayText(revision.text)}</pre>
                </li>
              ))}
            </ol>
          )}
        </section>

        <section aria-labelledby="docx-inert-parts-heading" data-testid="docx-inert-parts">
          <h3 id="docx-inert-parts-heading">Embedded objects / media / custom XML</h3>
          {inertParts.length === 0 ? <p>No such package parts were observed.</p> : (
            <ul>
              {inertParts.map((part) => (
                <li key={part.sequence}>
                  <code>{part.part_name}</code> — {part.classification} ({part.uncompressed_size} bytes)
                </li>
              ))}
            </ul>
          )}
        </section>

        <section aria-labelledby="docx-findings-heading" data-testid="docx-findings">
          <h3 id="docx-findings-heading">Document findings</h3>
          {analysis.findings.length === 0 ? <p>No DOCX structural findings.</p> : (
            <ol className="findings-list">
              {analysis.findings.map((finding) => (
                <li key={finding.sequence}>
                  <div className="finding-heading">
                    <code>{finding.kind}</code>
                    <span className="severity">{finding.severity}</span>
                  </div>
                  <p>{finding.description}</p>
                </li>
              ))}
            </ol>
          )}
        </section>
      </div>

      <ProvenanceEvidenceList evidence={analysis.provenance} testId="docx-provenance" />

      <div data-testid="docx-channels">
        <TextChannel title="Headers" channels={analysis.text_channels.headers} />
        <TextChannel title="Footers" channels={analysis.text_channels.footers} />
        <TextChannel title="Footnotes" channels={analysis.text_channels.footnotes} />
        <TextChannel title="Endnotes" channels={analysis.text_channels.endnotes} />
        <TextChannel title="Comments channel" channels={analysis.text_channels.comments} />
        {analysis.text_channels.deleted_content !== '' && (
          <section aria-label="Deleted content channel">
            <h3>Deleted content channel</h3>
            <pre className="decoded-output" data-testid="docx-deleted-content">
              {analysis.text_channels.deleted_content}
            </pre>
          </section>
        )}
      </div>
    </section>
  )
}
