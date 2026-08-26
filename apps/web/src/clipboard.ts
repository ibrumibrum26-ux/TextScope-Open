import {
  analyzeHtmlBytes,
  analyzeTextBytes,
  compareClipboardRepresentations as compareClipboardRepresentationsCore,
  HtmlAnalysisClientError,
  TextIngestionClientError,
} from './wasm/client.ts'
import type {
  HtmlAnalysis,
  ProvenanceEvidence,
  TextSourceAnalysis,
} from './wasm/contract.ts'
import { MAX_LOCAL_INPUT_BYTES } from './resource-limits.ts'

export const MAX_CLIPBOARD_BYTES = MAX_LOCAL_INPUT_BYTES

export type ClipboardRepresentationKind = 'plain_text' | 'html' | 'other_mime'
export type ClipboardComparisonState =
  | 'equivalent'
  | 'technically_different'
  | 'content_different'
  | 'needs_review'

export interface ClipboardRepresentation {
  kind: ClipboardRepresentationKind
  mime_type: string
  byte_length: number | null
  text: string | null
}

export interface ClipboardMetadata {
  advertised_types: string[]
  representation_order: string[]
  plain_text_present: boolean
  html_present: boolean
}

export type ClipboardFindingCode = 'unsupported_mime' | 'missing_readable_representation'

export interface ClipboardFinding {
  code: ClipboardFindingCode
  message: string
  severity: 'info' | 'review'
}

export interface ClipboardComparison {
  state: ClipboardComparisonState
  plain_byte_length: number | null
  html_text_byte_length: number | null
}

export interface ClipboardAnalysis {
  representations: ClipboardRepresentation[]
  metadata: ClipboardMetadata
  findings: ClipboardFinding[]
  plain_text: string | null
  plain_analysis: TextSourceAnalysis | null
  html_source: string | null
  html_analysis: HtmlAnalysis | null
  html_error: string | null
  selected_text: string
  comparison: ClipboardComparison
  provenance: ProvenanceEvidence[]
}

export type ClipboardErrorCode = 'unavailable' | 'denied' | 'empty' | 'too_large' | 'read_failed'

export class ClipboardReadError extends Error {
  readonly code: ClipboardErrorCode

  constructor(code: ClipboardErrorCode, message: string) {
    super(message)
    this.name = 'ClipboardReadError'
    this.code = code
  }
}

export interface ClipboardPayload {
  representations: ClipboardRepresentation[]
  plain_text: string | null
  html_source: string | null
}

function clipboardMetadata(
  plainText: string | null,
  htmlSource: string | null,
  representations: ClipboardRepresentation[],
): ClipboardMetadata {
  return {
    advertised_types: representations.map((representation) => representation.mime_type),
    representation_order: representations.map((representation) => representation.kind),
    plain_text_present: plainText !== null,
    html_present: htmlSource !== null,
  }
}

function clipboardFindings(representations: ClipboardRepresentation[]): ClipboardFinding[] {
  const unsupported = representations
    .filter((representation) => representation.kind === 'other_mime')
    .map((representation) => representation.mime_type)
  const findings: ClipboardFinding[] = []
  if (unsupported.length > 0) {
    findings.push({
      code: 'unsupported_mime',
      message: `Unsupported advertised MIME types were retained as observations: ${unsupported.join(', ')}.`,
      severity: 'info',
    })
  }
  if (!representations.some((representation) => representation.kind !== 'other_mime')) {
    findings.push({
      code: 'missing_readable_representation',
      message: 'No readable text/plain or text/html representation was available.',
      severity: 'review',
    })
  }
  return findings
}

function utf8Bytes(value: string): Uint8Array {
  return new TextEncoder().encode(value)
}

function ensureClipboardSize(total: number): void {
  if (!Number.isSafeInteger(total) || total < 0 || total > MAX_CLIPBOARD_BYTES) {
    throw new ClipboardReadError(
      'too_large',
      'The clipboard content exceeds the 10 MiB local interface limit.',
    )
  }
}

function decodeUtf8(bytes: Uint8Array): string {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes)
  } catch {
    throw new ClipboardReadError('read_failed', 'The clipboard text could not be decoded safely.')
  }
}

function provenance(
  plainText: string | null,
  htmlSource: string | null,
  representations: ClipboardRepresentation[],
): ProvenanceEvidence[] {
  const records: ProvenanceEvidence[] = []
  for (const representation of representations) {
    const source =
      representation.kind === 'plain_text'
        ? 'clipboard_plain_text'
        : representation.kind === 'html'
          ? 'clipboard_html'
          : 'clipboard_other_mime'
    records.push({
      kind: 'clipboard_representation',
      source,
      subject: representation.mime_type,
      value:
        representation.byte_length === null
          ? 'advertised; byte length unavailable without reading unsupported MIME'
          : `${representation.byte_length} bytes observed`,
      level: 'observation',
      verification: 'observed',
    })
  }
  records.push({
    kind: 'attribution',
    source:
      plainText !== null
        ? 'clipboard_plain_text'
        : htmlSource !== null
          ? 'clipboard_html'
          : 'clipboard_other_mime',
    subject: 'attribution',
    value: 'No independently verifiable attribution evidence is available.',
    level: 'attribution',
    verification: 'unsupported',
  })
  return records
}

export async function analyzeClipboardPayload(payload: ClipboardPayload): Promise<ClipboardAnalysis> {
  const plainBytes = payload.plain_text === null ? null : utf8Bytes(payload.plain_text)
  const htmlBytes = payload.html_source === null ? null : utf8Bytes(payload.html_source)
  const totalBytes = (plainBytes?.byteLength ?? 0) + (htmlBytes?.byteLength ?? 0)
  ensureClipboardSize(totalBytes)

  let plainAnalysis: TextSourceAnalysis | null = null
  let htmlAnalysis: HtmlAnalysis | null = null
  let htmlError: string | null = null
  if (plainBytes !== null) {
    try {
      plainAnalysis = await analyzeTextBytes(plainBytes, 'plain_text')
    } catch (error) {
      if (error instanceof TextIngestionClientError) {
        throw new ClipboardReadError('read_failed', error.message)
      }
      throw error
    }
  }
  if (htmlBytes !== null) {
    try {
      htmlAnalysis = await analyzeHtmlBytes(htmlBytes)
    } catch (error) {
      htmlError =
        error instanceof HtmlAnalysisClientError
          ? error.message
          : 'The HTML clipboard representation could not be analyzed safely.'
    }
  }

  const extractedHtmlText = htmlAnalysis?.text_channels.document_text ?? null
  const comparison = await compareClipboardRepresentationsCore(payload.plain_text, extractedHtmlText)
  const selectedText = payload.plain_text ?? extractedHtmlText ?? ''
  return {
    representations: payload.representations,
    metadata: clipboardMetadata(payload.plain_text, payload.html_source, payload.representations),
    findings: clipboardFindings(payload.representations),
    plain_text: payload.plain_text,
    plain_analysis: plainAnalysis,
    html_source: payload.html_source,
    html_analysis: htmlAnalysis,
    html_error: htmlError,
    selected_text: selectedText,
    comparison,
    provenance: provenance(payload.plain_text, payload.html_source, payload.representations),
  }
}

function classifyType(mimeType: string): ClipboardRepresentationKind {
  if (mimeType === 'text/plain') return 'plain_text'
  if (mimeType === 'text/html') return 'html'
  return 'other_mime'
}

export function clipboardPayloadFromStrings(
  plainText: string | null,
  htmlSource: string | null,
  otherMimeTypes: string[],
): ClipboardPayload {
  const representations: ClipboardRepresentation[] = []
  if (plainText !== null) {
    representations.push({
      kind: 'plain_text',
      mime_type: 'text/plain',
      byte_length: utf8Bytes(plainText).byteLength,
      text: plainText,
    })
  }
  if (htmlSource !== null) {
    representations.push({
      kind: 'html',
      mime_type: 'text/html',
      byte_length: utf8Bytes(htmlSource).byteLength,
      text: htmlSource,
    })
  }
  for (const mimeType of otherMimeTypes) {
    if (mimeType !== 'text/plain' && mimeType !== 'text/html') {
      representations.push({ kind: 'other_mime', mime_type: mimeType, byte_length: null, text: null })
    }
  }
  ensureClipboardSize(
    (plainText === null ? 0 : utf8Bytes(plainText).byteLength) +
      (htmlSource === null ? 0 : utf8Bytes(htmlSource).byteLength),
  )
  return { representations, plain_text: plainText, html_source: htmlSource }
}

export async function analyzeClipboardDataTransfer(data: DataTransfer): Promise<ClipboardAnalysis> {
  const types = Array.from(data.types)
  let plainText: string | null = null
  let htmlSource: string | null = null
  try {
    plainText = types.includes('text/plain') ? data.getData('text/plain') : null
    htmlSource = types.includes('text/html') ? data.getData('text/html') : null
  } catch {
    throw new ClipboardReadError('read_failed', 'The pasted clipboard representation could not be read.')
  }
  if (plainText === null && htmlSource === null && types.length === 0) {
    throw new ClipboardReadError('empty', 'The clipboard did not advertise a readable text representation.')
  }
  const payload = clipboardPayloadFromStrings(plainText, htmlSource, types)
  ensureClipboardSize(
    (plainText === null ? 0 : utf8Bytes(plainText).byteLength) +
      (htmlSource === null ? 0 : utf8Bytes(htmlSource).byteLength),
  )
  return analyzeClipboardPayload(payload)
}

async function readClipboardItem(item: ClipboardItem): Promise<ClipboardPayload> {
  const representations: ClipboardRepresentation[] = []
  let plainText: string | null = null
  let htmlSource: string | null = null
  let totalBytes = 0
  for (const mimeType of item.types) {
    const kind = classifyType(mimeType)
    if (kind === 'other_mime') {
      representations.push({ kind, mime_type: mimeType, byte_length: null, text: null })
      continue
    }
    let blob: Blob
    try {
      blob = await item.getType(mimeType)
    } catch {
      throw new ClipboardReadError('read_failed', 'The clipboard representation could not be read.')
    }
    if (!Number.isSafeInteger(blob.size) || blob.size < 0) {
      throw new ClipboardReadError('read_failed', 'The clipboard reported an invalid representation size.')
    }
    totalBytes += blob.size
    ensureClipboardSize(totalBytes)
    let bytes: Uint8Array
    try {
      bytes = new Uint8Array(await blob.arrayBuffer())
    } catch {
      throw new ClipboardReadError('read_failed', 'The clipboard representation could not be read.')
    }
    const value = decodeUtf8(bytes)
    representations.push({ kind, mime_type: mimeType, byte_length: bytes.byteLength, text: value })
    if (kind === 'plain_text' && plainText === null) plainText = value
    if (kind === 'html' && htmlSource === null) htmlSource = value
  }
  return { representations, plain_text: plainText, html_source: htmlSource }
}

export async function readClipboard(): Promise<ClipboardAnalysis> {
  const clipboard = typeof navigator === 'undefined' ? undefined : navigator.clipboard
  if (clipboard === undefined) {
    throw new ClipboardReadError('unavailable', 'This browser does not expose the Clipboard API.')
  }
  const extended = clipboard as Clipboard & { read?: () => Promise<ClipboardItem[]> }
  if (typeof extended.read === 'function') {
    let items: ClipboardItem[]
    try {
      items = await extended.read()
    } catch (error) {
      const name = error instanceof DOMException ? error.name : ''
      throw new ClipboardReadError(
        name === 'NotAllowedError' || name === 'SecurityError' ? 'denied' : 'read_failed',
        name === 'NotAllowedError' || name === 'SecurityError'
          ? 'Clipboard permission was denied. Use the paste area as a safe fallback.'
          : 'The browser could not read the clipboard.',
      )
    }
    if (items.length === 0) {
      throw new ClipboardReadError('empty', 'The clipboard is empty or has no readable items.')
    }
    const payloads = await Promise.all(items.map(readClipboardItem))
    const allRepresentations = payloads.flatMap((item) => item.representations)
    ensureClipboardSize(
      allRepresentations.reduce(
        (total, representation) => total + (representation.byte_length ?? 0),
        0,
      ),
    )
    const plainText = payloads.find((item) => item.plain_text !== null)?.plain_text ?? null
    const htmlSource = payloads.find((item) => item.html_source !== null)?.html_source ?? null
    return analyzeClipboardPayload({
      representations: allRepresentations,
      plain_text: plainText,
      html_source: htmlSource,
    })
  }
  if (typeof clipboard.readText === 'function') {
    try {
      const text = await clipboard.readText()
      if (text === '') {
        throw new ClipboardReadError('empty', 'The clipboard contains no text.')
      }
      return analyzeClipboardPayload(clipboardPayloadFromStrings(text, null, []))
    } catch (error) {
      if (error instanceof ClipboardReadError) throw error
      throw new ClipboardReadError('denied', 'Clipboard permission was denied. Use the paste area as a safe fallback.')
    }
  }
  throw new ClipboardReadError('unavailable', 'This browser does not expose a readable Clipboard API.')
}

export async function analyzeClipboardStrings(
  plainText: string | null,
  htmlSource: string | null,
  otherMimeTypes: string[] = [],
): Promise<ClipboardAnalysis> {
  if (plainText === null && htmlSource === null && otherMimeTypes.length === 0) {
    throw new ClipboardReadError('empty', 'No readable clipboard representation was supplied.')
  }
  return analyzeClipboardPayload(clipboardPayloadFromStrings(plainText, htmlSource, otherMimeTypes))
}
