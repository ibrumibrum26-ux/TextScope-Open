import initializeWasm, {
  analyze_docx_bytes as analyzeDocxBytesExport,
  analyze_html_bytes as analyzeHtmlBytesExport,
  analyze_pdf_bytes as analyzePdfBytesExport,
  analyze_text_bytes as analyzeTextBytesExport,
  compare_clipboard_representations as compareClipboardRepresentationsExport,
  available_clean_profiles as availableCleanProfilesExport,
  clean_text as cleanTextExport,
  reveal_text as revealTextExport,
  scan_text as scanTextExport,
  tcs_version as tcsVersionExport,
} from './generated/textscope_wasm.js'

import {
  parseCleanProfiles,
  parseCleanResult,
  parseHtmlAnalysisResponse,
  parseScanResult,
  parseTextSourceAnalysisResponse,
  type CleanProfile,
  type CleanProfileDefinition,
  type CleanResult,
  type HtmlAnalysis,
  type HtmlAnalysisErrorCode,
  type ScanResult,
  type TextIngestionErrorCode,
  type TextSourceAnalysis,
  type TextSourceKind,
} from './contract.ts'
import {
  parseDocxAnalysisResponse,
  type DocxAnalysis,
  type DocxAnalysisErrorCode,
} from './docx-contract.ts'
import {
  parsePdfAnalysisResponse,
  type PdfAnalysis,
  type PdfAnalysisErrorCode,
} from './pdf-contract.ts'

type WasmClientErrorCode =
  | 'INITIALIZATION_FAILED'
  | 'SCAN_FAILED'
  | 'REVEAL_FAILED'
  | 'CLEAN_FAILED'
  | 'PROFILES_FAILED'
  | 'VERSION_FAILED'
  | 'SOURCE_ANALYSIS_FAILED'
  | 'HTML_ANALYSIS_FAILED'
  | 'DOCX_ANALYSIS_FAILED'
  | 'PDF_ANALYSIS_FAILED'
  | 'CLIPBOARD_COMPARISON_FAILED'

export class WasmClientError extends Error {
  readonly code: WasmClientErrorCode

  constructor(code: WasmClientErrorCode) {
    super('The local TextScope WASM module could not complete the request.')
    this.name = 'WasmClientError'
    this.code = code
  }
}

export class TextIngestionClientError extends Error {
  readonly code: TextIngestionErrorCode

  constructor(code: TextIngestionErrorCode, message: string) {
    super(message)
    this.name = 'TextIngestionClientError'
    this.code = code
  }
}

export class HtmlAnalysisClientError extends Error {
  readonly code: HtmlAnalysisErrorCode

  constructor(code: HtmlAnalysisErrorCode, message: string) {
    super(message)
    this.name = 'HtmlAnalysisClientError'
    this.code = code
  }
}

export class DocxAnalysisClientError extends Error {
  readonly code: DocxAnalysisErrorCode

  constructor(code: DocxAnalysisErrorCode, message: string) {
    super(message)
    this.name = 'DocxAnalysisClientError'
    this.code = code
  }
}

export class PdfAnalysisClientError extends Error {
  readonly code: PdfAnalysisErrorCode

  constructor(code: PdfAnalysisErrorCode, message: string) {
    super(message)
    this.name = 'PdfAnalysisClientError'
    this.code = code
  }
}

let initialization: Promise<void> | null = null

async function ensureWasmInitialized(): Promise<void> {
  if (initialization === null) {
    initialization = initializeWasm()
      .then(() => undefined)
      .catch(() => {
        initialization = null
        throw new WasmClientError('INITIALIZATION_FAILED')
      })
  }

  await initialization
}

export async function analyzeText(text: string): Promise<ScanResult> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = scanTextExport(text)
  } catch {
    throw new WasmClientError('SCAN_FAILED')
  }

  return parseScanResult(serialized)
}

export interface ClipboardComparisonResult {
  state: 'equivalent' | 'technically_different' | 'content_different' | 'needs_review'
  plain_byte_length: number | null
  html_text_byte_length: number | null
}

export async function compareClipboardRepresentations(
  plainText: string | null,
  htmlText: string | null,
): Promise<ClipboardComparisonResult> {
  await ensureWasmInitialized()
  let serialized: string
  try {
    serialized = compareClipboardRepresentationsExport(plainText ?? undefined, htmlText ?? undefined)
  } catch {
    throw new WasmClientError('CLIPBOARD_COMPARISON_FAILED')
  }
  let parsed: unknown
  try {
    parsed = JSON.parse(serialized) as unknown
  } catch {
    throw new WasmClientError('CLIPBOARD_COMPARISON_FAILED')
  }
  if (
    typeof parsed !== 'object' ||
    parsed === null ||
    Array.isArray(parsed) ||
    !('state' in parsed) ||
    !('plain_byte_length' in parsed) ||
    !('html_text_byte_length' in parsed) ||
    typeof parsed.state !== 'string' ||
    !['equivalent', 'technically_different', 'content_different', 'needs_review'].includes(parsed.state) ||
    (parsed.plain_byte_length !== null &&
      (typeof parsed.plain_byte_length !== 'number' ||
        !Number.isSafeInteger(parsed.plain_byte_length) ||
        parsed.plain_byte_length < 0)) ||
    (parsed.html_text_byte_length !== null &&
      (typeof parsed.html_text_byte_length !== 'number' ||
        !Number.isSafeInteger(parsed.html_text_byte_length) ||
        parsed.html_text_byte_length < 0))
  ) {
    throw new WasmClientError('CLIPBOARD_COMPARISON_FAILED')
  }
  return parsed as ClipboardComparisonResult
}

export async function revealText(text: string): Promise<string> {
  await ensureWasmInitialized()

  try {
    return revealTextExport(text)
  } catch {
    throw new WasmClientError('REVEAL_FAILED')
  }
}

export async function cleanText(text: string, profile: CleanProfile): Promise<CleanResult> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = cleanTextExport(text, profile)
  } catch {
    throw new WasmClientError('CLEAN_FAILED')
  }

  return parseCleanResult(serialized)
}

export async function getAvailableCleanProfiles(): Promise<CleanProfileDefinition[]> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = availableCleanProfilesExport()
  } catch {
    throw new WasmClientError('PROFILES_FAILED')
  }

  return parseCleanProfiles(serialized)
}

export async function getTcsVersion(): Promise<string> {
  await ensureWasmInitialized()

  try {
    return tcsVersionExport()
  } catch {
    throw new WasmClientError('VERSION_FAILED')
  }
}

export async function analyzeTextBytes(
  bytes: Uint8Array,
  sourceKind: TextSourceKind,
): Promise<TextSourceAnalysis> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = analyzeTextBytesExport(bytes, sourceKind)
  } catch {
    throw new WasmClientError('SOURCE_ANALYSIS_FAILED')
  }

  const response = parseTextSourceAnalysisResponse(serialized)
  if (response.status === 'error') {
    throw new TextIngestionClientError(response.error.code, response.error.message)
  }

  return response.analysis
}

export async function analyzeHtmlBytes(bytes: Uint8Array): Promise<HtmlAnalysis> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = analyzeHtmlBytesExport(bytes)
  } catch {
    throw new WasmClientError('HTML_ANALYSIS_FAILED')
  }

  const response = parseHtmlAnalysisResponse(serialized)
  if (response.status === 'error') {
    throw new HtmlAnalysisClientError(response.error.code, response.error.message)
  }

  return response.analysis
}

export async function analyzeDocxBytes(bytes: Uint8Array): Promise<DocxAnalysis> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = analyzeDocxBytesExport(bytes)
  } catch {
    throw new WasmClientError('DOCX_ANALYSIS_FAILED')
  }

  const response = parseDocxAnalysisResponse(serialized)
  if (response.status === 'error') {
    throw new DocxAnalysisClientError(response.error.code, response.error.message)
  }

  return response.analysis
}

export async function analyzePdfBytes(bytes: Uint8Array): Promise<PdfAnalysis> {
  await ensureWasmInitialized()

  let serialized: string
  try {
    serialized = analyzePdfBytesExport(bytes)
  } catch {
    throw new WasmClientError('PDF_ANALYSIS_FAILED')
  }

  const response = parsePdfAnalysisResponse(serialized)
  if (response.status === 'error') {
    throw new PdfAnalysisClientError(response.error.code, response.error.message)
  }

  return response.analysis
}
