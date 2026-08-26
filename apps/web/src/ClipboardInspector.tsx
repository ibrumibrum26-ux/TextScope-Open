import { ProvenanceEvidenceList } from './ProvenanceEvidenceList.tsx'
import type { ClipboardAnalysis } from './clipboard.ts'

function display(value: string | null): string {
  return value === null || value === '' ? '(empty text)' : value
}

export function ClipboardInspector({ analysis }: { analysis: ClipboardAnalysis }) {
  return (
    <section aria-labelledby="clipboard-inspection-heading" data-testid="clipboard-inspector">
      <div className="section-heading-row">
        <div>
          <p className="eyebrow dark">Explicit local clipboard inspection</p>
          <h2 id="clipboard-inspection-heading">Rich Clipboard</h2>
        </div>
        <p className="source-unchanged">Clipboard snapshot unchanged</p>
      </div>
      <p className="safety-note">
        TextScope read the clipboard only after an explicit user action or paste event. It does not
        poll, retain clipboard history, write clipboard contents, log text, or contact a network.
      </p>

      <section aria-labelledby="clipboard-representations-heading">
        <h3 id="clipboard-representations-heading">Advertised representations</h3>
        {analysis.representations.length === 0 ? (
          <p>No clipboard representations were observed.</p>
        ) : (
          <ul>
            {analysis.representations.map((representation, index) => (
              <li key={`${representation.mime_type}-${index}`}>
                <code>{representation.mime_type}</code> — {representation.kind};{' '}
                {representation.byte_length === null
                  ? 'byte length not read (unsupported MIME)'
                  : `${representation.byte_length} bytes`}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="clipboard-comparison-heading">
        <h3 id="clipboard-comparison-heading">Deterministic representation comparison</h3>
        <p data-testid="clipboard-comparison-state">
          <strong>{analysis.comparison.state}</strong>. Exact text equality is equivalent; line
          ending-only differences are technical; other differences remain content differences.
        </p>
      </section>

      {analysis.findings.length > 0 && (
        <section aria-labelledby="clipboard-findings-heading">
          <h3 id="clipboard-findings-heading">Clipboard observations</h3>
          <ul>
            {analysis.findings.map((finding) => (
              <li key={finding.code}>
                <strong>{finding.severity}</strong>: {finding.message}
              </li>
            ))}
          </ul>
        </section>
      )}

      <div className="inspector-grid">
        <section aria-labelledby="clipboard-plain-heading" data-testid="clipboard-plain-panel">
          <h3 id="clipboard-plain-heading">text/plain</h3>
          {analysis.plain_analysis === null ? (
            <p>No plain-text representation was available.</p>
          ) : (
            <>
              <p className="result-note">
                This representation is the default Scan/Reveal/Clean channel when present.
              </p>
              <pre className="decoded-output">{display(analysis.plain_text)}</pre>
            </>
          )}
        </section>

        <section aria-labelledby="clipboard-html-heading" data-testid="clipboard-html-panel">
          <h3 id="clipboard-html-heading">text/html</h3>
          {analysis.html_analysis === null ? (
            <p>{analysis.html_error ?? 'No HTML representation was available.'}</p>
          ) : (
            <>
              <p className="result-note">
                HTML stays inert. The Rust HTML adapter supplies structural observations and
                extracted document text; the browser never renders this source.
              </p>
              <dl className="source-metadata">
                <div>
                  <dt>Elements</dt>
                  <dd>{analysis.html_analysis.summary.parsed_elements}</dd>
                </div>
                <div>
                  <dt>Attributes</dt>
                  <dd>{analysis.html_analysis.summary.attributes}</dd>
                </div>
                <div>
                  <dt>Findings</dt>
                  <dd>{analysis.html_analysis.findings.length}</dd>
                </div>
              </dl>
              <h4>Extracted document text</h4>
              <pre className="decoded-output">
                {display(analysis.html_analysis.text_channels.document_text)}
              </pre>
            </>
          )}
        </section>
      </div>

      <ProvenanceEvidenceList evidence={analysis.provenance} testId="clipboard-provenance" />
    </section>
  )
}
