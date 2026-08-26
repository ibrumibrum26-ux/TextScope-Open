import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type ClipboardEvent as ReactClipboardEvent,
  type FormEvent,
} from 'react'

import {
  loadLocalDocxFile,
  loadLocalHtmlFile,
  loadLocalPdfFile,
  loadLocalTextFile,
  localFileErrorMessage,
  type LocalFileMetadata,
  type LocalDocxFileMetadata,
  type LocalHtmlFileMetadata,
  type LocalPdfFileMetadata,
} from './local-file'
import { DocxInspector } from './DocxInspector.tsx'
import { PdfInspector } from './PdfInspector.tsx'
import { ClipboardInspector } from './ClipboardInspector.tsx'
import { ProvenanceEvidenceList } from './ProvenanceEvidenceList.tsx'
import {
  analyzeClipboardDataTransfer,
  ClipboardReadError,
  readClipboard,
  type ClipboardAnalysis,
} from './clipboard.ts'

import {
  analyzeText,
  cleanText,
  getAvailableCleanProfiles,
  getTcsVersion,
  revealText,
} from './wasm/client'
import {
  summarizeScanResult,
  type CharacterRecord,
  type CleanProfile,
  type CleanProfileDefinition,
  type CleanResult,
  type HtmlAnalysis,
  type ScanResult,
  type SourceCharacterMapping,
  type TextSourceAnalysis,
} from './wasm/contract'
import type { DocxAnalysis } from './wasm/docx-contract.ts'
import type { PdfAnalysis } from './wasm/pdf-contract.ts'

type UiPhase = 'idle' | 'working' | 'ready' | 'error'
type InputMode = 'paste' | 'clipboard' | 'file' | 'html' | 'docx' | 'pdf'

const analysisErrorMessage =
  'TextScope could not complete the local analysis. Please verify the WASM setup and try again.'
const cleanErrorMessage =
  'TextScope could not complete the local clean operation. The original text was not changed.'
const profileErrorMessage =
  'TextScope could not load the local TCS profile registry. Cleaning is unavailable.'

function displaySegment(value: string): string {
  return value === '' ? '(empty text)' : value
}

function encodingLabel(encoding: TextSourceAnalysis['decode']['encoding']): string {
  switch (encoding) {
    case 'utf_8':
      return 'UTF-8'
    case 'utf_16_le':
      return 'UTF-16 little-endian'
    case 'utf_16_be':
      return 'UTF-16 big-endian'
  }
}

function bomLabel(bom: TextSourceAnalysis['decode']['bom']): string {
  switch (bom) {
    case 'none':
      return 'None'
    case 'utf_8':
      return 'UTF-8 BOM'
    case 'utf_16_le':
      return 'UTF-16 LE BOM'
    case 'utf_16_be':
      return 'UTF-16 BE BOM'
  }
}

function evidenceLabel(evidence: TextSourceAnalysis['decode']['encoding_evidence']): string {
  switch (evidence) {
    case 'empty_input':
      return 'Empty input rule'
    case 'strict_utf_8':
      return 'Strict UTF-8 validation'
    case 'byte_order_mark':
      return 'Byte-order mark'
  }
}

function CharacterRow({
  record,
  sourceMapping,
}: {
  record: CharacterRecord
  sourceMapping?: SourceCharacterMapping
}) {
  return (
    <tr>
      <td>{record.code_point_index}</td>
      <td>{record.utf16_offset}</td>
      <td>{record.utf8_byte_offset}</td>
      <td>
        <code>{record.code_point_label}</code>
      </td>
      <td>{record.unicode_category}</td>
      <td>{record.script}</td>
      {sourceMapping !== undefined && (
        <td data-testid={`source-byte-range-${record.code_point_index}`}>
          [{sourceMapping.source_byte_range.start}, {sourceMapping.source_byte_range.end})
        </td>
      )}
    </tr>
  )
}

export function App() {
  const [inputMode, setInputMode] = useState<InputMode>('paste')
  const [text, setText] = useState('')
  const [fileMetadata, setFileMetadata] = useState<LocalFileMetadata | null>(null)
  const [sourceAnalysis, setSourceAnalysis] = useState<TextSourceAnalysis | null>(null)
  const [htmlFileMetadata, setHtmlFileMetadata] = useState<LocalHtmlFileMetadata | null>(null)
  const [htmlAnalysis, setHtmlAnalysis] = useState<HtmlAnalysis | null>(null)
  const [docxFileMetadata, setDocxFileMetadata] = useState<LocalDocxFileMetadata | null>(null)
  const [docxAnalysis, setDocxAnalysis] = useState<DocxAnalysis | null>(null)
  const [pdfFileMetadata, setPdfFileMetadata] = useState<LocalPdfFileMetadata | null>(null)
  const [pdfAnalysis, setPdfAnalysis] = useState<PdfAnalysis | null>(null)
  const [clipboardAnalysis, setClipboardAnalysis] = useState<ClipboardAnalysis | null>(null)
  const [result, setResult] = useState<ScanResult | null>(null)
  const [revealed, setRevealed] = useState<string | null>(null)
  const [phase, setPhase] = useState<UiPhase>('idle')
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  const [profiles, setProfiles] = useState<CleanProfileDefinition[]>([])
  const [selectedProfile, setSelectedProfile] = useState<CleanProfile | ''>('')
  const [cleanResult, setCleanResult] = useState<CleanResult | null>(null)
  const [tcsVersion, setTcsVersion] = useState<string | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const fileOperationId = useRef(0)

  useEffect(() => {
    let current = true

    Promise.all([getAvailableCleanProfiles(), getTcsVersion()])
      .then(([definitions, version]) => {
        if (!current) {
          return
        }

        const supportedProfiles = definitions.filter((definition) => definition.supported)
        const defaultProfile = supportedProfiles[0]
        if (defaultProfile === undefined) {
          throw new Error('No supported clean profile was returned by the Rust core.')
        }

        setProfiles(supportedProfiles)
        setSelectedProfile(defaultProfile.id)
        setTcsVersion(version)
      })
      .catch(() => {
        if (current) {
          setProfiles([])
          setSelectedProfile('')
          setTcsVersion(null)
          setPhase('error')
          setErrorMessage(profileErrorMessage)
        }
      })

    return () => {
      current = false
    }
  }, [])

  const summary = useMemo(
    () => (result === null ? null : summarizeScanResult(result)),
    [result],
  )

  const resetResults = (nextText: string) => {
    setText(nextText)
    setResult(null)
    setRevealed(null)
    setCleanResult(null)
    setErrorMessage(null)
    setPhase('idle')
  }

  const resetInput = (nextMode: InputMode) => {
    fileOperationId.current += 1
    setInputMode(nextMode)
    setFileMetadata(null)
    setSourceAnalysis(null)
    setHtmlFileMetadata(null)
    setHtmlAnalysis(null)
    setDocxFileMetadata(null)
    setDocxAnalysis(null)
    setPdfFileMetadata(null)
    setPdfAnalysis(null)
    setClipboardAnalysis(null)
    resetResults('')

    if (fileInputRef.current !== null) {
      fileInputRef.current.value = ''
    }
  }

  const handleFileChange = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.currentTarget.files?.[0]
    const operationId = fileOperationId.current + 1
    fileOperationId.current = operationId
    setFileMetadata(null)
    setSourceAnalysis(null)
    setHtmlFileMetadata(null)
    setHtmlAnalysis(null)
    setDocxFileMetadata(null)
    setDocxAnalysis(null)
    setPdfFileMetadata(null)
    setPdfAnalysis(null)
    setClipboardAnalysis(null)
    resetResults('')

    if (file === undefined) {
      return
    }

    setPhase('working')

    try {
      if (inputMode === 'docx') {
        const loaded = await loadLocalDocxFile(file)
        if (fileOperationId.current !== operationId) {
          return
        }

        setDocxFileMetadata(loaded.metadata)
        setDocxAnalysis(loaded.analysis)
        setText(loaded.analysis.text_channels.document_text)
        setPhase('idle')
        return
      }

      if (inputMode === 'html') {
        const loaded = await loadLocalHtmlFile(file)
        if (fileOperationId.current !== operationId) {
          return
        }

        setHtmlFileMetadata(loaded.metadata)
        setHtmlAnalysis(loaded.analysis)
        setText(loaded.analysis.text_channels.document_text)
        setPhase('idle')
        return
      }

      if (inputMode === 'pdf') {
        const loaded = await loadLocalPdfFile(file)
        if (fileOperationId.current !== operationId) {
          return
        }

        setPdfFileMetadata(loaded.metadata)
        setPdfAnalysis(loaded.analysis)
        setText(loaded.analysis.document_text)
        setPhase('idle')
        return
      }

      const loaded = await loadLocalTextFile(file)
      if (fileOperationId.current !== operationId) {
        return
      }

      setFileMetadata(loaded.metadata)
      setSourceAnalysis(loaded.analysis)
      setText(loaded.analysis.decode.decoded_text)
      setPhase('idle')
    } catch (error) {
      if (fileOperationId.current !== operationId) {
        return
      }

      if (fileInputRef.current !== null) {
        fileInputRef.current.value = ''
      }
      setPhase('error')
      setErrorMessage(localFileErrorMessage(error))
    }
  }

  const handleRemoveFile = () => {
    fileOperationId.current += 1
    setFileMetadata(null)
    setSourceAnalysis(null)
    setHtmlFileMetadata(null)
    setHtmlAnalysis(null)
    setDocxFileMetadata(null)
    setDocxAnalysis(null)
    setPdfFileMetadata(null)
    setPdfAnalysis(null)
    setClipboardAnalysis(null)
    resetResults('')

    if (fileInputRef.current !== null) {
      fileInputRef.current.value = ''
    }
  }

  const applyClipboardAnalysis = (analysis: ClipboardAnalysis) => {
    setClipboardAnalysis(analysis)
    setText(analysis.selected_text)
    setResult(null)
    setRevealed(null)
    setCleanResult(null)
    setErrorMessage(null)
    setPhase('idle')
  }

  const handleReadClipboard = async () => {
    setPhase('working')
    setErrorMessage(null)
    setClipboardAnalysis(null)
    setText('')
    setResult(null)
    setRevealed(null)
    setCleanResult(null)
    try {
      applyClipboardAnalysis(await readClipboard())
    } catch (error) {
      setPhase('error')
      if (error instanceof ClipboardReadError) {
        setErrorMessage(error.message)
      } else {
        setErrorMessage('TextScope could not read the clipboard locally.')
      }
    }
  }

  const handleClipboardPaste = async (event: ReactClipboardEvent<HTMLTextAreaElement>) => {
    event.preventDefault()
    setPhase('working')
    setErrorMessage(null)
    setClipboardAnalysis(null)
    setText('')
    setResult(null)
    setRevealed(null)
    setCleanResult(null)
    try {
      applyClipboardAnalysis(await analyzeClipboardDataTransfer(event.clipboardData))
    } catch (error) {
      setPhase('error')
      setErrorMessage(
        error instanceof ClipboardReadError
          ? error.message
          : 'TextScope could not analyze the pasted clipboard data locally.',
      )
    }
  }

  const handleAnalyze = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setPhase('working')
    setErrorMessage(null)
    setRevealed(null)

    try {
      setResult(
        sourceAnalysis?.scan ??
          htmlAnalysis?.scan ??
          docxAnalysis?.scan ??
          pdfAnalysis?.scan ??
          clipboardAnalysis?.plain_analysis?.scan ??
          clipboardAnalysis?.html_analysis?.scan ??
          (await analyzeText(text)),
      )
      setPhase('ready')
    } catch {
      setResult(null)
      setPhase('error')
      setErrorMessage(analysisErrorMessage)
    }
  }

  const handleReveal = async () => {
    setPhase('working')
    setErrorMessage(null)

    try {
      setRevealed(await revealText(text))
      setPhase(result === null ? 'idle' : 'ready')
    } catch {
      setRevealed(null)
      setPhase('error')
      setErrorMessage(analysisErrorMessage)
    }
  }

  const handleProfileChange = (profileId: string) => {
    const definition = profiles.find(
      (candidate) => candidate.id === profileId && candidate.supported,
    )
    if (definition === undefined) {
      return
    }

    setSelectedProfile(definition.id)
    setCleanResult(null)
    setErrorMessage(null)
    setPhase(result === null ? 'idle' : 'ready')
  }

  const handleClean = async () => {
    if (selectedProfile === '') {
      setPhase('error')
      setErrorMessage(profileErrorMessage)
      return
    }

    setPhase('working')
    setErrorMessage(null)
    setCleanResult(null)

    try {
      setCleanResult(await cleanText(text, selectedProfile))
      setPhase('ready')
    } catch {
      setPhase('error')
      setErrorMessage(cleanErrorMessage)
    }
  }

  const isWorking = phase === 'working'
  const canOperate =
    inputMode === 'paste' ||
    clipboardAnalysis !== null ||
    sourceAnalysis !== null ||
    htmlAnalysis !== null ||
    docxAnalysis !== null ||
    pdfAnalysis !== null
  const selectedProfileDefinition = profiles.find(
    (definition) => definition.id === selectedProfile,
  )

  return (
    <>
      <header className="site-header">
        <div className="page-width">
          <p className="eyebrow">Local-first text inspection</p>
          <h1>TextScope</h1>
          <p className="lede">
            Inspect Unicode text locally through the authoritative Rust core.
            Your text is not sent to an application endpoint.
          </p>
        </div>
      </header>

      <main className="page-width">
        <form className="analysis-form" onSubmit={handleAnalyze}>
          <fieldset className="input-mode-picker">
            <legend>Input source</legend>
            <label>
              <input
                type="radio"
                name="input-mode"
                value="paste"
                checked={inputMode === 'paste'}
                onChange={() => resetInput('paste')}
                disabled={isWorking}
              />
              Paste text
            </label>
            <label>
              <input
                type="radio"
                name="input-mode"
                value="clipboard"
                checked={inputMode === 'clipboard'}
                onChange={() => resetInput('clipboard')}
                disabled={isWorking}
              />
              Rich Clipboard
            </label>
            <label>
              <input
                type="radio"
                name="input-mode"
                value="file"
                checked={inputMode === 'file'}
                onChange={() => resetInput('file')}
                disabled={isWorking}
              />
              Local text file
            </label>
            <label>
              <input
                type="radio"
                name="input-mode"
                value="html"
                checked={inputMode === 'html'}
                onChange={() => resetInput('html')}
                disabled={isWorking}
              />
              HTML file
            </label>
            <label>
              <input
                type="radio"
                name="input-mode"
                value="docx"
                checked={inputMode === 'docx'}
                onChange={() => resetInput('docx')}
                disabled={isWorking}
              />
              DOCX file
            </label>
            <label>
              <input
                type="radio"
                name="input-mode"
                value="pdf"
                checked={inputMode === 'pdf'}
                onChange={() => resetInput('pdf')}
                disabled={isWorking}
              />
              PDF file
            </label>
          </fieldset>

          {inputMode === 'paste' ? (
            <>
              <label htmlFor="text-input">Text to analyze</label>
              <textarea
                id="text-input"
                name="text"
                rows={10}
                value={text}
                onChange={(event) => resetResults(event.target.value)}
                placeholder="Paste or type text here"
                spellCheck={false}
              />
            </>
          ) : inputMode === 'clipboard' ? (
            <div className="clipboard-input-panel">
              <label htmlFor="clipboard-input">Clipboard text preview and paste fallback</label>
              <textarea
                id="clipboard-input"
                name="clipboard-text"
                rows={10}
                value={text}
                onChange={(event) => {
                  setClipboardAnalysis(null)
                  resetResults(event.target.value)
                }}
                onPaste={handleClipboardPaste}
                placeholder="Click Read Clipboard, or paste here to inspect text/plain and text/html"
                spellCheck={false}
              />
              <div className="button-row">
                <button type="button" onClick={handleReadClipboard} disabled={isWorking}>
                  {isWorking ? 'Reading…' : 'Read Clipboard'}
                </button>
              </div>
              <p className="input-help">
                Clipboard access is explicit and one-shot. Unsupported MIME types are listed but
                never read; the 10 MiB aggregate limit is checked before WASM.
              </p>
            </div>
          ) : (
            <div className="file-input-panel">
              <label htmlFor="file-input">
                {inputMode === 'html'
                  ? 'Choose HTML file'
                  : inputMode === 'docx'
                    ? 'Choose DOCX file'
                    : inputMode === 'pdf'
                      ? 'Choose PDF file'
                      : 'Choose TXT or Markdown file'}
              </label>
              <input
                ref={fileInputRef}
                id="file-input"
                name="local-file"
                type="file"
                accept={
                  inputMode === 'html'
                    ? '.html,.htm'
                    : inputMode === 'docx'
                      ? '.docx'
                      : inputMode === 'pdf'
                        ? '.pdf'
                        : '.txt,.md,.markdown'
                }
                onChange={handleFileChange}
                disabled={isWorking}
              />
              <p className="input-help">
                {inputMode === 'html'
                  ? 'Supported extensions: .html and .htm.'
                  : inputMode === 'docx'
                    ? 'Supported extension: .docx. Macro-enabled .docm is rejected.'
                    : inputMode === 'pdf'
                      ? 'Supported extension: .pdf. The PDF is treated as untrusted binary data.'
                      : 'Supported extensions: .txt, .md, and .markdown.'}{' '}
                Maximum size: 10 MiB. The browser-reported MIME type is advisory only.
              </p>
              {(sourceAnalysis !== null || htmlAnalysis !== null || docxAnalysis !== null || pdfAnalysis !== null) && (
                <button
                  type="button"
                  className="secondary"
                  onClick={handleRemoveFile}
                  disabled={isWorking}
                >
                  Remove file
                </button>
              )}
            </div>
          )}

          <div className="button-row">
            <button type="submit" disabled={isWorking || !canOperate}>
              {isWorking ? 'Working…' : 'Analyze'}
            </button>
            <button
              type="button"
              className="secondary"
              onClick={handleReveal}
              disabled={isWorking || !canOperate}
            >
              Reveal
            </button>
          </div>
        </form>

        {sourceAnalysis !== null && fileMetadata !== null && (
          <section aria-labelledby="file-source-heading" data-testid="file-source-panel">
            <div className="section-heading-row">
              <div>
                <p className="eyebrow dark">Strict local byte ingestion</p>
                <h2 id="file-source-heading">Local file source</h2>
              </div>
              <p className="source-unchanged" data-testid="source-unchanged">
                Source file unchanged
              </p>
            </div>
            <dl className="source-metadata">
              <div>
                <dt>File name</dt>
                <dd data-testid="file-name">{fileMetadata.file_name}</dd>
              </div>
              <div>
                <dt>Adapter</dt>
                <dd data-testid="source-kind">{fileMetadata.source_kind_label}</dd>
              </div>
              <div>
                <dt>Raw bytes</dt>
                <dd data-testid="raw-byte-length">{sourceAnalysis.raw_byte_length}</dd>
              </div>
              <div>
                <dt>Browser MIME (advisory)</dt>
                <dd data-testid="browser-mime">
                  {fileMetadata.browser_reported_mime_type || 'Not reported'}
                </dd>
              </div>
              <div>
                <dt>Encoding</dt>
                <dd data-testid="source-encoding">
                  {encodingLabel(sourceAnalysis.decode.encoding)}
                </dd>
              </div>
              <div>
                <dt>Encoding evidence</dt>
                <dd>{evidenceLabel(sourceAnalysis.decode.encoding_evidence)}</dd>
              </div>
              <div>
                <dt>BOM</dt>
                <dd data-testid="source-bom">{bomLabel(sourceAnalysis.decode.bom)}</dd>
              </div>
              <div>
                <dt>Decode status</dt>
                <dd data-testid="decode-status">Decoded strictly</dd>
              </div>
              <div>
                <dt>Characters</dt>
                <dd data-testid="decoded-characters">
                  {sourceAnalysis.scan.characters.length}
                </dd>
              </div>
              <div>
                <dt>Findings</dt>
                <dd data-testid="decoded-findings">{sourceAnalysis.scan.findings.length}</dd>
              </div>
            </dl>
            <p className="safety-note">
              TextScope read a local snapshot and did not write to the source file. File paths are
              not retained or displayed.
            </p>
            <h3>Decoded text</h3>
            <pre className="decoded-output" data-testid="decoded-text">
              {displaySegment(sourceAnalysis.decode.decoded_text)}
            </pre>
            {fileMetadata.source_kind === 'markdown' && (
              <p className="result-note">Markdown is displayed and analyzed as literal text.</p>
            )}
          </section>
        )}

        {htmlAnalysis !== null && htmlFileMetadata !== null && (
          <section aria-labelledby="html-source-heading" data-testid="html-source-panel">
            <div className="section-heading-row">
              <div>
                <p className="eyebrow dark">Inert Rust HTML forensics</p>
                <h2 id="html-source-heading">Local HTML source</h2>
              </div>
              <p className="source-unchanged" data-testid="source-unchanged">
                Source file unchanged
              </p>
            </div>

            <dl className="source-metadata">
              <div>
                <dt>File name</dt>
                <dd data-testid="file-name">{htmlFileMetadata.file_name}</dd>
              </div>
              <div>
                <dt>Adapter</dt>
                <dd data-testid="source-kind">HTML</dd>
              </div>
              <div>
                <dt>Raw bytes</dt>
                <dd data-testid="raw-byte-length">{htmlAnalysis.raw_byte_length}</dd>
              </div>
              <div>
                <dt>Browser MIME (advisory)</dt>
                <dd data-testid="browser-mime">
                  {htmlFileMetadata.browser_reported_mime_type || 'Not reported'}
                </dd>
              </div>
              <div>
                <dt>Encoding</dt>
                <dd data-testid="source-encoding">
                  {encodingLabel(htmlAnalysis.decode.encoding)}
                </dd>
              </div>
              <div>
                <dt>BOM</dt>
                <dd data-testid="source-bom">{bomLabel(htmlAnalysis.decode.bom)}</dd>
              </div>
              <div>
                <dt>Title</dt>
                <dd data-testid="html-title">{htmlAnalysis.metadata.title ?? 'Not declared'}</dd>
              </div>
              <div>
                <dt>Language</dt>
                <dd data-testid="html-lang">{htmlAnalysis.metadata.lang ?? 'Not declared'}</dd>
              </div>
              <div>
                <dt>Elements (parsed)</dt>
                <dd data-testid="html-elements">{htmlAnalysis.summary.parsed_elements}</dd>
              </div>
              <div>
                <dt>Attributes</dt>
                <dd data-testid="html-attributes">{htmlAnalysis.summary.attributes}</dd>
              </div>
              <div>
                <dt>Text segments</dt>
                <dd data-testid="html-text-segments">{htmlAnalysis.summary.text_segments}</dd>
              </div>
              <div>
                <dt>Comments</dt>
                <dd data-testid="html-comments-count">{htmlAnalysis.summary.comments}</dd>
              </div>
              <div>
                <dt>Scripts</dt>
                <dd data-testid="html-scripts-count">{htmlAnalysis.summary.script_blocks}</dd>
              </div>
              <div>
                <dt>External references</dt>
                <dd data-testid="html-external-count">
                  {htmlAnalysis.summary.external_references}
                </dd>
              </div>
            </dl>

            <p className="safety-note">
              The Rust Core parsed this snapshot as untrusted data. TextScope did not render the
              HTML, execute active content, resolve references, or write to the file.
            </p>

            <section aria-labelledby="extracted-document-text-heading">
              <h3 id="extracted-document-text-heading">Extracted Document Text</h3>
              <p className="result-note">
                This text was extracted from the HTML5 structure and does not represent the
                original file bytes.
              </p>
              <pre className="decoded-output" data-testid="extracted-document-text">
                {displaySegment(htmlAnalysis.text_channels.document_text)}
              </pre>
            </section>

            <div className="inspector-grid">
              <section aria-labelledby="html-metadata-heading" data-testid="html-metadata">
                <h3 id="html-metadata-heading">Document metadata</h3>
                <p>Observed declarations only; no attribution was externally verified.</p>
                {htmlAnalysis.metadata.meta_declarations.length === 0 &&
                htmlAnalysis.metadata.canonical_links.length === 0 &&
                htmlAnalysis.metadata.base_hrefs.length === 0 ? (
                  <p>No metadata declarations were observed.</p>
                ) : (
                  <ul>
                    {htmlAnalysis.metadata.meta_declarations.map((declaration) => (
                      <li key={`meta-${declaration.sequence}`}>
                        <code>
                          {declaration.charset !== null
                            ? `charset=${declaration.charset}`
                            : `${declaration.name ?? declaration.property ?? 'meta'}=${declaration.content ?? ''}`}
                        </code>
                      </li>
                    ))}
                    {htmlAnalysis.metadata.canonical_links.map((value, index) => (
                      <li key={`canonical-${index}`}>
                        canonical: <code>{value}</code>
                      </li>
                    ))}
                    {htmlAnalysis.metadata.base_hrefs.map((value, index) => (
                      <li key={`base-${index}`}>
                        base href: <code>{value}</code>
                      </li>
                    ))}
                  </ul>
                )}
              </section>

              <section
                aria-labelledby="external-references-heading"
                data-testid="external-references"
              >
                <h3 id="external-references-heading">External references</h3>
                {htmlAnalysis.external_references.length === 0 ? (
                  <p>No reference-bearing attributes were observed.</p>
                ) : (
                  <ol>
                    {htmlAnalysis.external_references.map((reference) => (
                      <li key={reference.sequence}>
                        <code>{`${reference.element}[${reference.attribute}]`}</code>{' '}
                        <span className="severity">{reference.kind}</span>
                        <p>{reference.value}</p>
                      </li>
                    ))}
                  </ol>
                )}
              </section>

              <section aria-labelledby="html-comments-heading" data-testid="html-comments">
                <h3 id="html-comments-heading">Comments</h3>
                {htmlAnalysis.comments.length === 0 ? (
                  <p>No HTML comments were observed.</p>
                ) : (
                  <ol>
                    {htmlAnalysis.comments.map((comment) => (
                      <li key={comment.sequence}>
                        <pre>{displaySegment(comment.text)}</pre>
                      </li>
                    ))}
                  </ol>
                )}
              </section>

              <section aria-labelledby="html-findings-heading" data-testid="html-findings">
                <h3 id="html-findings-heading">HTML findings</h3>
                {htmlAnalysis.findings.length === 0 ? (
                  <p>No HTML structural findings.</p>
                ) : (
                  <ol className="findings-list">
                    {htmlAnalysis.findings.map((finding) => (
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

            <ProvenanceEvidenceList evidence={htmlAnalysis.provenance} testId="html-provenance" />

            {htmlAnalysis.text_channels.active_content_text !== '' && (
              <section aria-labelledby="active-content-heading">
                <h3 id="active-content-heading">Inert script content</h3>
                <pre className="decoded-output" data-testid="html-active-content">
                  {htmlAnalysis.text_channels.active_content_text}
                </pre>
              </section>
            )}
            {htmlAnalysis.text_channels.style_text !== '' && (
              <section aria-labelledby="style-content-heading">
                <h3 id="style-content-heading">Inert style content</h3>
                <pre className="decoded-output" data-testid="html-style-content">
                  {htmlAnalysis.text_channels.style_text}
                </pre>
              </section>
            )}
          </section>
        )}

        {clipboardAnalysis !== null && <ClipboardInspector analysis={clipboardAnalysis} />}

        {docxAnalysis !== null && docxFileMetadata !== null && (
          <DocxInspector metadata={docxFileMetadata} analysis={docxAnalysis} />
        )}

        {pdfAnalysis !== null && pdfFileMetadata !== null && (
          <PdfInspector metadata={pdfFileMetadata} analysis={pdfAnalysis} />
        )}

        <section className="clean-controls" aria-labelledby="clean-controls-heading">
          <div className="section-heading-row">
            <div>
              <p className="eyebrow dark">Versioned local policy</p>
              <h2 id="clean-controls-heading">
                {inputMode === 'html'
                  ? 'Clean extracted document text'
                  : inputMode === 'docx'
                    ? 'Clean extracted document text'
                    : inputMode === 'pdf'
                      ? 'Clean extracted document text'
                      : inputMode === 'clipboard'
                        ? 'Clean selected clipboard text'
                      : inputMode === 'file'
                        ? 'Clean decoded file text'
                        : 'Clean pasted text'}
              </h2>
            </div>
            <p className="version-badge" data-testid="tcs-version">
              TCS {tcsVersion ?? 'loading…'}
            </p>
          </div>
          <label htmlFor="clean-profile">Clean profile</label>
          <select
            id="clean-profile"
            value={selectedProfile}
            onChange={(event) => handleProfileChange(event.target.value)}
            disabled={isWorking || selectedProfile === '' || !canOperate}
          >
            {selectedProfile === '' && <option value="">Loading profiles…</option>}
            {profiles.map((definition) => (
              <option key={definition.id} value={definition.id}>
                {definition.label}
              </option>
            ))}
          </select>
          {selectedProfileDefinition !== undefined && (
            <p className="profile-description" data-testid="profile-description">
              {selectedProfileDefinition.description}
            </p>
          )}
          <p className="safety-note">
            Cleaning is explicit. The original{' '}
            {inputMode === 'paste'
              ? 'textarea'
              : inputMode === 'clipboard'
                ? 'clipboard representations'
                : 'source file'} remains unchanged, and every
            applied transformation is reported below.
          </p>
          <button
            type="button"
            onClick={handleClean}
            disabled={isWorking || selectedProfile === '' || !canOperate}
          >
            {isWorking ? 'Working…' : 'Clean'}
          </button>
        </section>

        <div className="status" aria-live="polite">
          {phase === 'idle' && result === null && (
            <p>
              {clipboardAnalysis !== null
                ? 'Clipboard representations analyzed locally. Choose an operation for the selected text.'
                : sourceAnalysis === null
                  ? htmlAnalysis === null
                    ? docxAnalysis === null
                      ? pdfAnalysis === null
                        ? 'No analysis has been run.'
                        : 'PDF parsed locally. Choose an operation for the extracted document text.'
                      : 'DOCX parsed locally. Choose an operation for the extracted document text.'
                    : 'HTML parsed locally. Choose an operation for the extracted document text.'
                  : 'File decoded and scanned locally. Choose an operation to inspect the result.'}
            </p>
          )}
          {phase === 'working' && <p>Running local operation…</p>}
          {phase === 'error' && errorMessage !== null && (
            <p className="error" role="alert">
              {errorMessage}
            </p>
          )}
        </div>

        {revealed !== null && (
          <section aria-labelledby="reveal-heading">
            <h2 id="reveal-heading">Reveal</h2>
            <pre className="reveal-output" data-testid="reveal-output">
              {revealed === '' ? '(empty text)' : revealed}
            </pre>
          </section>
        )}

        {cleanResult !== null && (
          <div className="clean-results" data-testid="clean-results">
            <section aria-labelledby="clean-text-heading">
              <div className="section-heading-row">
                <h2 id="clean-text-heading">Clean Text</h2>
                <p className="version-badge">{cleanResult.profile}</p>
              </div>
              <pre className="clean-output" data-testid="clean-text">
                {displaySegment(cleanResult.clean_text)}
              </pre>
              <p className="result-note">Original input was preserved separately.</p>
            </section>

            <section aria-labelledby="verification-heading" data-testid="verification-report">
              <h2 id="verification-heading">Verification</h2>
              <dl className="verification-grid">
                <div>
                  <dt>Status</dt>
                  <dd data-testid="verification-status">
                    {cleanResult.verification.verification_status}
                  </dd>
                </div>
                <div>
                  <dt>Transformations</dt>
                  <dd data-testid="clean-transformations-count">
                    {cleanResult.verification.applied_transformations}
                  </dd>
                </div>
                <div>
                  <dt>Remaining findings</dt>
                  <dd data-testid="remaining-findings-count">
                    {cleanResult.verification.remaining_findings.length}
                  </dd>
                </div>
                <div>
                  <dt>Content preservation</dt>
                  <dd data-testid="content-preservation-status">
                    {cleanResult.verification.content_preservation_status}
                  </dd>
                </div>
                <div>
                  <dt>Independent clean rescan</dt>
                  <dd data-testid="clean-rescan-status">
                    {cleanResult.verification.checks.clean_rescan_completed ? 'Completed' : 'Failed'}
                  </dd>
                </div>
              </dl>
              <p>{cleanResult.content_preservation_diff.reason}</p>
              {cleanResult.verification.remaining_findings.length > 0 && (
                <div className="remaining-findings">
                  <h3>Remaining technical findings</h3>
                  <ul>
                    {cleanResult.verification.remaining_findings.map((finding) => (
                      <li key={`${finding.code_point_index}-${finding.kind}`}>
                        <code>{finding.kind}</code>: {finding.description}
                      </li>
                    ))}
                  </ul>
                </div>
              )}
            </section>

            <section aria-labelledby="technical-diff-heading">
              <h2 id="technical-diff-heading">Technical Diff</h2>
              <dl className="verification-grid compact">
                <div>
                  <dt>Changed</dt>
                  <dd>{cleanResult.technical_diff.changed ? 'Yes' : 'No'}</dd>
                </div>
                <div>
                  <dt>Original UTF-8 bytes</dt>
                  <dd>{cleanResult.technical_diff.original_utf8_bytes}</dd>
                </div>
                <div>
                  <dt>Clean UTF-8 bytes</dt>
                  <dd>{cleanResult.technical_diff.clean_utf8_bytes}</dd>
                </div>
              </dl>
            </section>

            <section aria-labelledby="manifest-heading" data-testid="transformation-manifest">
              <h2 id="manifest-heading">Transformation Manifest</h2>
              {cleanResult.manifest.transformations.length === 0 ? (
                <p>No transformations were applied.</p>
              ) : (
                <ol className="transformations-list">
                  {cleanResult.manifest.transformations.map((transformation) => (
                    <li key={transformation.sequence} data-testid="transformation-entry">
                      <div className="finding-heading">
                        <code>{transformation.rule_id}</code>
                        <span className="severity">{transformation.operation}</span>
                      </div>
                      <p>
                        {`Original code points [${transformation.original_code_point_range.start}, ${transformation.original_code_point_range.end}); UTF-8 bytes [${transformation.original_utf8_byte_range.start}, ${transformation.original_utf8_byte_range.end}).`}
                      </p>
                      <p>{transformation.reason}</p>
                      <div className="before-after-grid">
                        <div>
                          <h3>Before</h3>
                          <pre>{displaySegment(transformation.before)}</pre>
                        </div>
                        <div>
                          <h3>After</h3>
                          <pre>{displaySegment(transformation.after)}</pre>
                        </div>
                      </div>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          </div>
        )}

        {result !== null && summary !== null && (
          <div className="results" data-testid="analysis-results">
            <section aria-labelledby="overview-heading">
              <h2 id="overview-heading">Overview</h2>
              <dl className="metrics">
                <div>
                  <dt>Characters</dt>
                  <dd data-testid="characters-count">{summary.characters}</dd>
                </div>
                <div>
                  <dt>Invisible</dt>
                  <dd data-testid="invisible-count">{summary.invisible}</dd>
                </div>
                <div>
                  <dt>Unusual whitespace</dt>
                  <dd data-testid="unusual-whitespace-count">{summary.unusualWhitespace}</dd>
                </div>
                <div>
                  <dt>Bidi controls</dt>
                  <dd data-testid="bidi-controls-count">{summary.bidiControls}</dd>
                </div>
              </dl>
            </section>

            <section aria-labelledby="normalization-heading">
              <h2 id="normalization-heading">Normalization</h2>
              <dl className="normalization-list">
                <div>
                  <dt>NFC</dt>
                  <dd>{result.normalization.nfc_would_change ? 'Would change' : 'Unchanged'}</dd>
                </div>
                <div>
                  <dt>NFD</dt>
                  <dd>{result.normalization.nfd_would_change ? 'Would change' : 'Unchanged'}</dd>
                </div>
                <div>
                  <dt>NFKC</dt>
                  <dd>{result.normalization.nfkc_would_change ? 'Would change' : 'Unchanged'}</dd>
                </div>
                <div>
                  <dt>NFKD</dt>
                  <dd>{result.normalization.nfkd_would_change ? 'Would change' : 'Unchanged'}</dd>
                </div>
              </dl>
            </section>

            <section aria-labelledby="characters-heading">
              <h2 id="characters-heading">Character records</h2>
              {result.characters.length === 0 ? (
                <p>The analyzed text is empty.</p>
              ) : (
                <div className="table-scroll" tabIndex={0}>
                  <table>
                    <caption>
                      Decoded-text offsets, Unicode properties
                      {sourceAnalysis === null
                        ? ' returned by the Rust core'
                        : ', and original source-byte ranges returned by the Rust core'}
                    </caption>
                    <thead>
                      <tr>
                        <th scope="col">Index</th>
                        <th scope="col">UTF-16</th>
                        <th scope="col">Decoded UTF-8</th>
                        <th scope="col">Code point</th>
                        <th scope="col">Category</th>
                        <th scope="col">Script</th>
                        {sourceAnalysis !== null && <th scope="col">Source bytes</th>}
                      </tr>
                    </thead>
                    <tbody>
                      {result.characters.map((record) => (
                        <CharacterRow
                          key={record.code_point_index}
                          record={record}
                          sourceMapping={
                            sourceAnalysis?.decode.source_mappings[record.code_point_index]
                          }
                        />
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </section>

            <section aria-labelledby="findings-heading">
              <h2 id="findings-heading">Technical findings</h2>
              {result.findings.length === 0 ? (
                <p>No technical findings.</p>
              ) : (
                <ol className="findings-list">
                  {result.findings.map((finding) => (
                    <li key={`${finding.code_point_index}-${finding.kind}`}>
                      <div className="finding-heading">
                        <code>{finding.kind}</code>
                        <span className="severity">{finding.severity}</span>
                      </div>
                      <p>
                        Code-point index {finding.code_point_index}; UTF-16 offset{' '}
                        {finding.utf16_offset}; decoded UTF-8 byte offset {finding.utf8_byte_offset}.
                      </p>
                      <p>{finding.description}</p>
                    </li>
                  ))}
                </ol>
              )}
            </section>
          </div>
        )}
      </main>
    </>
  )
}
