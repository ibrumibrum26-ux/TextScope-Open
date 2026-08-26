import type { ProvenanceEvidence } from './wasm/contract.ts'

export function ProvenanceEvidenceList({
  evidence,
  testId = 'provenance-evidence',
}: {
  evidence: ProvenanceEvidence[] | undefined
  testId?: string
}) {
  const records = evidence ?? []
  return (
    <section aria-labelledby={`${testId}-heading`} data-testid={testId}>
      <h3 id={`${testId}-heading`}>Provenance &amp; Metadata Evidence</h3>
      <p className="result-note">
        Observation, declaration, verification, and attribution are separate. TextScope does not
        infer authorship or intent from stored fields.
      </p>
      {records.length === 0 ? (
        <p>No provenance evidence was observed.</p>
      ) : (
        <div className="table-scroll" tabIndex={0}>
          <table>
            <caption>Observable evidence returned by the local Rust adapter</caption>
            <thead>
              <tr>
                <th scope="col">Evidence</th>
                <th scope="col">Source</th>
                <th scope="col">Status</th>
                <th scope="col">Value</th>
              </tr>
            </thead>
            <tbody>
              {records.map((record, index) => (
                <tr key={`${record.source}-${record.subject}-${index}`}>
                  <td>{record.subject || record.kind}</td>
                  <td>{record.source}</td>
                  <td>{record.verification}</td>
                  <td>{record.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  )
}
