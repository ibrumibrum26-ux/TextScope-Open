import {
  analyzeDocxBytes,
  analyzeHtmlBytes,
  analyzePdfBytes,
  DocxAnalysisClientError,
  analyzeTextBytes,
  HtmlAnalysisClientError,
  PdfAnalysisClientError,
  TextIngestionClientError,
  WasmClientError,
} from './wasm/client.ts'
import type { HtmlAnalysis, TextSourceAnalysis, TextSourceKind } from './wasm/contract.ts'
import type { DocxAnalysis } from './wasm/docx-contract.ts'
import type { PdfAnalysis } from './wasm/pdf-contract.ts'
import { MAX_LOCAL_INPUT_BYTES } from './resource-limits.ts'

/** UI guard applied before File.arrayBuffer(); HTML repeats it in the Rust Core. */
export { MAX_LOCAL_INPUT_BYTES }
export const MAX_LOCAL_TEXT_FILE_BYTES = MAX_LOCAL_INPUT_BYTES
export const MAX_LOCAL_HTML_FILE_BYTES = MAX_LOCAL_INPUT_BYTES
export const MAX_LOCAL_DOCX_FILE_BYTES = MAX_LOCAL_INPUT_BYTES
export const MAX_LOCAL_PDF_FILE_BYTES = MAX_LOCAL_INPUT_BYTES

export type LocalFileErrorCode = 'unsupported_file_type' | 'file_too_large' | 'file_read_failed'

export class LocalFileInputError extends Error {
  readonly code: LocalFileErrorCode

  constructor(code: LocalFileErrorCode, message: string) {
    super(message)
    this.name = 'LocalFileInputError'
    this.code = code
  }
}

export interface LocalFileMetadata {
  file_name: string
  extension: '.txt' | '.md' | '.markdown'
  browser_reported_mime_type: string
  byte_size: number
  source_kind: TextSourceKind
  source_kind_label: 'TXT' | 'Markdown'
}

export interface LocalFileAnalysis {
  metadata: LocalFileMetadata
  analysis: TextSourceAnalysis
}

export interface LocalHtmlFileMetadata {
  file_name: string
  extension: '.html' | '.htm'
  browser_reported_mime_type: string
  byte_size: number
  source_kind_label: 'HTML'
}

export interface LocalHtmlFileAnalysis {
  metadata: LocalHtmlFileMetadata
  analysis: HtmlAnalysis
}

export interface LocalDocxFileMetadata {
  file_name: string
  extension: '.docx'
  browser_reported_mime_type: string
  byte_size: number
  source_kind_label: 'DOCX'
}

export interface LocalDocxFileAnalysis {
  metadata: LocalDocxFileMetadata
  analysis: DocxAnalysis
}

export interface LocalPdfFileMetadata {
  file_name: string
  extension: '.pdf'
  browser_reported_mime_type: string
  byte_size: number
  source_kind_label: 'PDF'
}

export interface LocalPdfFileAnalysis {
  metadata: LocalPdfFileMetadata
  analysis: PdfAnalysis
}

interface BrowserFileMetadata {
  readonly name: string
  readonly size: number
  readonly type: string
}

const supportedExtensions: Record<
  LocalFileMetadata['extension'],
  Pick<LocalFileMetadata, 'source_kind' | 'source_kind_label'>
> = {
  '.txt': { source_kind: 'plain_text', source_kind_label: 'TXT' },
  '.md': { source_kind: 'markdown', source_kind_label: 'Markdown' },
  '.markdown': { source_kind: 'markdown', source_kind_label: 'Markdown' },
}

export function inspectLocalTextFile(file: BrowserFileMetadata): LocalFileMetadata {
  const dotIndex = file.name.lastIndexOf('.')
  const extension = (dotIndex >= 0 ? file.name.slice(dotIndex).toLowerCase() : '') as string
  const adapter = supportedExtensions[extension as LocalFileMetadata['extension']]

  if (adapter === undefined) {
    throw new LocalFileInputError(
      'unsupported_file_type',
      'Only .txt, .md, and .markdown files are supported in this TextScope phase.',
    )
  }

  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_LOCAL_TEXT_FILE_BYTES) {
    throw new LocalFileInputError(
      'file_too_large',
      'The selected file exceeds the 10 MiB local interface limit.',
    )
  }

  return {
    file_name: file.name,
    extension: extension as LocalFileMetadata['extension'],
    browser_reported_mime_type: file.type,
    byte_size: file.size,
    source_kind: adapter.source_kind,
    source_kind_label: adapter.source_kind_label,
  }
}

export function inspectLocalHtmlFile(file: BrowserFileMetadata): LocalHtmlFileMetadata {
  const dotIndex = file.name.lastIndexOf('.')
  const extension = (dotIndex >= 0 ? file.name.slice(dotIndex).toLowerCase() : '') as string

  if (extension !== '.html' && extension !== '.htm') {
    throw new LocalFileInputError(
      'unsupported_file_type',
      'Only .html and .htm files are supported in HTML File mode.',
    )
  }

  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_LOCAL_HTML_FILE_BYTES) {
    throw new LocalFileInputError(
      'file_too_large',
      'The selected file exceeds the 10 MiB local interface limit.',
    )
  }

  return {
    file_name: file.name,
    extension,
    browser_reported_mime_type: file.type,
    byte_size: file.size,
    source_kind_label: 'HTML',
  }
}

export function inspectLocalDocxFile(file: BrowserFileMetadata): LocalDocxFileMetadata {
  const dotIndex = file.name.lastIndexOf('.')
  const extension = (dotIndex >= 0 ? file.name.slice(dotIndex).toLowerCase() : '') as string

  if (extension !== '.docx') {
    throw new LocalFileInputError(
      'unsupported_file_type',
      'Only .docx files are supported in DOCX File mode. Macro-enabled .docm files are not supported.',
    )
  }

  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_LOCAL_DOCX_FILE_BYTES) {
    throw new LocalFileInputError(
      'file_too_large',
      'The selected file exceeds the 10 MiB local interface limit.',
    )
  }

  return {
    file_name: file.name,
    extension,
    browser_reported_mime_type: file.type,
    byte_size: file.size,
    source_kind_label: 'DOCX',
  }
}

export function inspectLocalPdfFile(file: BrowserFileMetadata): LocalPdfFileMetadata {
  const dotIndex = file.name.lastIndexOf('.')
  const extension = (dotIndex >= 0 ? file.name.slice(dotIndex).toLowerCase() : '') as string

  if (extension !== '.pdf') {
    throw new LocalFileInputError(
      'unsupported_file_type',
      'Only .pdf files are supported in PDF File mode.',
    )
  }

  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > MAX_LOCAL_PDF_FILE_BYTES) {
    throw new LocalFileInputError(
      'file_too_large',
      'The selected file exceeds the 10 MiB local interface limit.',
    )
  }

  return {
    file_name: file.name,
    extension: '.pdf',
    browser_reported_mime_type: file.type,
    byte_size: file.size,
    source_kind_label: 'PDF',
  }
}

export async function loadLocalTextFile(file: File): Promise<LocalFileAnalysis> {
  // Metadata and size are validated before File.arrayBuffer() is called.
  const metadata = inspectLocalTextFile(file)

  let buffer: ArrayBuffer
  try {
    buffer = await file.arrayBuffer()
  } catch {
    throw new LocalFileInputError(
      'file_read_failed',
      'The browser could not read the selected local file.',
    )
  }

  const analysis = await analyzeTextBytes(new Uint8Array(buffer), metadata.source_kind)
  return { metadata, analysis }
}

export async function loadLocalHtmlFile(file: File): Promise<LocalHtmlFileAnalysis> {
  // Metadata and size are validated before File.arrayBuffer() is called.
  const metadata = inspectLocalHtmlFile(file)

  let buffer: ArrayBuffer
  try {
    buffer = await file.arrayBuffer()
  } catch {
    throw new LocalFileInputError(
      'file_read_failed',
      'The browser could not read the selected local HTML file.',
    )
  }

  const analysis = await analyzeHtmlBytes(new Uint8Array(buffer))
  return { metadata, analysis }
}

export async function loadLocalDocxFile(file: File): Promise<LocalDocxFileAnalysis> {
  // The browser size guard runs before File.arrayBuffer(); the Rust Core repeats it.
  const metadata = inspectLocalDocxFile(file)

  let buffer: ArrayBuffer
  try {
    buffer = await file.arrayBuffer()
  } catch {
    throw new LocalFileInputError(
      'file_read_failed',
      'The browser could not read the selected local DOCX file.',
    )
  }

  const analysis = await analyzeDocxBytes(new Uint8Array(buffer))
  return { metadata, analysis }
}

export async function loadLocalPdfFile(file: File): Promise<LocalPdfFileAnalysis> {
  // The browser size guard runs before File.arrayBuffer(); the Rust Core repeats it.
  const metadata = inspectLocalPdfFile(file)

  let buffer: ArrayBuffer
  try {
    buffer = await file.arrayBuffer()
  } catch {
    throw new LocalFileInputError(
      'file_read_failed',
      'The browser could not read the selected local PDF file.',
    )
  }

  const analysis = await analyzePdfBytes(new Uint8Array(buffer))
  return { metadata, analysis }
}

export function localFileErrorMessage(error: unknown): string {
  if (
    error instanceof LocalFileInputError ||
    error instanceof DocxAnalysisClientError ||
    error instanceof HtmlAnalysisClientError ||
    error instanceof PdfAnalysisClientError ||
    error instanceof TextIngestionClientError ||
    error instanceof WasmClientError
  ) {
    return error.message
  }

  return 'TextScope could not process the selected local file.'
}
