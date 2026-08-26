import {
  BridgeContractError,
  isProvenanceEvidence,
  isScanResult,
  type ProvenanceEvidence,
  type ScanResult,
} from './contract.ts'

export type PdfXrefKind = 'table' | 'stream' | 'unknown'
export type PdfEncryptionStatus =
  | 'unencrypted'
  | 'encrypted_openable_with_empty_password'
  | 'password_required'
  | 'unsupported_encryption'
export type PdfTextExtractionStatus =
  | 'no_text_operators'
  | 'text_present_but_mapping_unavailable'
  | 'partially_extracted'
  | 'extracted'
export type PdfActionKind =
  | 'uri'
  | 'go_to'
  | 'go_to_remote'
  | 'launch'
  | 'javascript'
  | 'submit_form'
  | 'import_data'
  | 'rendition'
  | 'named'
  | 'other'
export type PdfExternalReferenceKind =
  | 'absolute_uri'
  | 'relative_reference'
  | 'file_like_target'
  | 'other'
export type PdfFindingKind =
  | 'incremental_updates_present'
  | 'encryption_present'
  | 'javascript_action_present'
  | 'launch_action_present'
  | 'external_uri_present'
  | 'submit_form_action_present'
  | 'embedded_file_present'
  | 'xfa_present'
  | 'form_present'
  | 'signature_structure_present'
  | 'explicit_non_painting_text_present'
  | 'malformed_structure'
  | 'resource_limit_exceeded'
  | 'text_mapping_unavailable'
  | 'no_text_layer'
  | 'metadata_declaration_conflict'
  | 'linearization_present'

export interface PdfVersion { major: number; minor: number; declared: string }
export interface PdfSourceReference {
  object_number: number | null
  generation: number | null
  raw_byte_offset: number | null
  context: string
}
export interface PdfObjectSummary {
  object_number: number
  generation: number
  kind: string
  has_stream: boolean
  compressed_in_object_stream: boolean
  raw_byte_offset: number | null
}
export interface PdfRevisionSummary {
  sequence: number
  startxref_offset: number | null
  xref_kind: PdfXrefKind
  prev_offset: number | null
  trailer_observed: boolean
  updated_object_ids: PdfSourceReference[]
}
export interface PdfBox { values: string[] }
export interface PdfPageSummary {
  page_index: number
  object: PdfSourceReference
  media_box: PdfBox | null
  crop_box: PdfBox | null
  rotate: number | null
  resources_present: boolean
  content_stream_count: number
  annotation_count: number
  text: string
}
export interface PdfImageRecord {
  sequence: number
  page_indices: number[]
  object: PdfSourceReference
  width: number | null
  height: number | null
  filter: string | null
  color_space: string | null
}
export interface PdfMetadataEntry { key: string; value: string; source: PdfSourceReference }
export interface PdfXmpMetadata {
  present: boolean
  well_formed_xml: boolean
  properties: PdfMetadataEntry[]
  source: PdfSourceReference | null
}
export interface PdfMetadataConflict {
  key: string
  info_value: string | null
  xmp_value: string | null
  description: string
}
export interface PdfMetadata {
  title: string | null
  author: string | null
  subject: string | null
  keywords: string | null
  creator: string | null
  producer: string | null
  creation_date: string | null
  modification_date: string | null
  trapped: string | null
  info_entries: PdfMetadataEntry[]
  xmp: PdfXmpMetadata | null
  conflicts: PdfMetadataConflict[]
}
export interface PdfActionRecord {
  sequence: number
  kind: PdfActionKind
  target: string | null
  source: PdfSourceReference
  context: string
}
export interface PdfAnnotationRecord {
  sequence: number
  page_index: number
  subtype: string
  contents: string | null
  title: string | null
  uri: string | null
  action: PdfActionKind | null
  flags: number | null
  source: PdfSourceReference
}
export interface PdfExternalReference {
  sequence: number
  value: string
  kind: PdfExternalReferenceKind
  source: PdfSourceReference
}
export interface PdfEmbeddedFileRecord {
  sequence: number
  declared_filename: string | null
  description: string | null
  mime_type: string | null
  size: number | null
  object: PdfSourceReference
  stream_object: PdfSourceReference | null
}
export interface PdfFormSummary {
  present: boolean
  field_count: number
  field_types: string[]
  need_appearances: boolean | null
  xfa_present: boolean
}
export interface PdfSignatureRecord {
  sequence: number
  source: PdfSourceReference
  byte_range_present: boolean
  contents_present: boolean
  sub_filter: string | null
  reason: string | null
  name: string | null
  modification_date: string | null
}
export interface PdfTextSegment {
  sequence: number
  page_index: number
  text: string
  font_reference: PdfSourceReference | null
  content_stream_object: PdfSourceReference | null
  operation_index: number
  source_object: PdfSourceReference | null
  extraction_status: PdfTextExtractionStatus
  explicit_non_painting: boolean
}
export interface PdfFinding {
  sequence: number
  kind: PdfFindingKind
  severity: 'informational' | 'caution'
  description: string
  source: PdfSourceReference | null
}
export interface PdfFontRecord {
  sequence: number
  page_indices: number[]
  resource_name: string
  source: PdfSourceReference
  subtype: string | null
  base_font: string | null
  encoding: string | null
  to_unicode_present: boolean
  mapping_status: PdfTextExtractionStatus
}
export interface PdfDocumentSummary {
  raw_byte_length: number
  version: PdfVersion | null
  binary_marker_present: boolean
  eof_marker_present: boolean
  startxref_count: number
  xref_kind: PdfXrefKind
  trailer_count: number
  object_count: number
  stream_count: number
  object_stream_count: number
  compressed_object_count: number
  page_count: number
  font_count: number
  image_count: number
  annotation_count: number
  action_count: number
  embedded_file_count: number
  form_present: boolean
  metadata_present: boolean
  javascript_present: boolean
  linearized: boolean
  encryption: PdfEncryptionStatus
}
export interface PdfAnalysis {
  summary: PdfDocumentSummary
  objects: PdfObjectSummary[]
  revisions: PdfRevisionSummary[]
  metadata: PdfMetadata
  provenance?: ProvenanceEvidence[]
  pages: PdfPageSummary[]
  images: PdfImageRecord[]
  fonts: PdfFontRecord[]
  actions: PdfActionRecord[]
  annotations: PdfAnnotationRecord[]
  external_references: PdfExternalReference[]
  embedded_files: PdfEmbeddedFileRecord[]
  forms: PdfFormSummary
  signatures: PdfSignatureRecord[]
  text_segments: PdfTextSegment[]
  page_text: string[]
  document_text: string
  text_extraction_status: PdfTextExtractionStatus
  findings: PdfFinding[]
  scan: ScanResult
}
export type PdfResourceLimit =
  | 'source_bytes' | 'objects' | 'pages' | 'decoded_stream_bytes' | 'individual_stream_bytes' | 'decompression_ratio'
  | 'reference_depth' | 'content_operations' | 'text_segments' | 'annotations' | 'actions'
  | 'embedded_files' | 'metadata_bytes' | 'findings'
export type PdfAnalysisErrorCode =
  | 'empty_input' | 'non_pdf_bytes' | 'invalid_header' | 'encrypted_pdf_unsupported'
  | 'password_required' | 'unsupported_encryption' | 'malformed_structure'
  | 'resource_limit_exceeded'
export interface PdfAnalysisError {
  code: PdfAnalysisErrorCode
  message: string
  resource_limit: PdfResourceLimit | null
}
export type PdfAnalysisResponse =
  | { status: 'success'; analysis: PdfAnalysis }
  | { status: 'error'; error: PdfAnalysisError }

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}
function integer(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}
function signedInteger(value: unknown): value is number {
  return Number.isSafeInteger(value)
}
function nullable<T>(value: unknown, predicate: (value: unknown) => value is T): value is T | null {
  return value === null || predicate(value)
}
function oneOf<T extends string>(value: unknown, values: readonly T[]): value is T {
  return typeof value === 'string' && values.includes(value as T)
}
const xrefKinds = ['table', 'stream', 'unknown'] as const
const encryptions = ['unencrypted', 'encrypted_openable_with_empty_password', 'password_required', 'unsupported_encryption'] as const
const statuses = ['no_text_operators', 'text_present_but_mapping_unavailable', 'partially_extracted', 'extracted'] as const
const actionKinds = ['uri', 'go_to', 'go_to_remote', 'launch', 'javascript', 'submit_form', 'import_data', 'rendition', 'named', 'other'] as const
const externalKinds = ['absolute_uri', 'relative_reference', 'file_like_target', 'other'] as const
const findingKinds = [
  'incremental_updates_present', 'encryption_present', 'javascript_action_present', 'launch_action_present',
  'external_uri_present', 'submit_form_action_present', 'embedded_file_present', 'xfa_present', 'form_present',
  'signature_structure_present', 'explicit_non_painting_text_present', 'malformed_structure', 'resource_limit_exceeded',
  'text_mapping_unavailable', 'no_text_layer', 'metadata_declaration_conflict', 'linearization_present',
] as const
const limits = [
  'source_bytes', 'objects', 'pages', 'decoded_stream_bytes', 'individual_stream_bytes', 'decompression_ratio', 'reference_depth',
  'content_operations', 'text_segments', 'annotations', 'actions', 'embedded_files', 'metadata_bytes', 'findings',
] as const
function source(value: unknown): value is PdfSourceReference {
  return isObject(value) && nullable(value.object_number, integer) && nullable(value.generation, integer) &&
    nullable(value.raw_byte_offset, integer) && typeof value.context === 'string'
}
function version(value: unknown): value is PdfVersion {
  return isObject(value) && integer(value.major) && integer(value.minor) && typeof value.declared === 'string'
}
function metadataEntry(value: unknown): value is PdfMetadataEntry {
  return isObject(value) && typeof value.key === 'string' && typeof value.value === 'string' && source(value.source)
}
function metadata(value: unknown): value is PdfMetadata {
  return isObject(value) && nullable(value.title, (v): v is string => typeof v === 'string') &&
    nullable(value.author, (v): v is string => typeof v === 'string') && nullable(value.subject, (v): v is string => typeof v === 'string') &&
    nullable(value.keywords, (v): v is string => typeof v === 'string') && nullable(value.creator, (v): v is string => typeof v === 'string') &&
    nullable(value.producer, (v): v is string => typeof v === 'string') && nullable(value.creation_date, (v): v is string => typeof v === 'string') &&
    nullable(value.modification_date, (v): v is string => typeof v === 'string') && nullable(value.trapped, (v): v is string => typeof v === 'string') &&
    Array.isArray(value.info_entries) && value.info_entries.every(metadataEntry) && Array.isArray(value.conflicts) &&
    value.conflicts.every((entry) => isObject(entry) && typeof entry.key === 'string' && nullable(entry.info_value, (v): v is string => typeof v === 'string') && nullable(entry.xmp_value, (v): v is string => typeof v === 'string') && typeof entry.description === 'string') &&
    (value.xmp === null || (isObject(value.xmp) && typeof value.xmp.present === 'boolean' && typeof value.xmp.well_formed_xml === 'boolean' && Array.isArray(value.xmp.properties) && value.xmp.properties.every(metadataEntry) && (value.xmp.source === null || source(value.xmp.source))))
}
function page(value: unknown): value is PdfPageSummary {
  return isObject(value) && integer(value.page_index) && source(value.object) && nullable(value.media_box, (box): box is PdfBox => isObject(box) && Array.isArray(box.values) && box.values.every((v) => typeof v === 'string')) &&
    nullable(value.crop_box, (box): box is PdfBox => isObject(box) && Array.isArray(box.values) && box.values.every((v) => typeof v === 'string')) &&
    nullable(value.rotate, signedInteger) && typeof value.resources_present === 'boolean' && integer(value.content_stream_count) && integer(value.annotation_count) && typeof value.text === 'string'
}
function image(value: unknown): value is PdfImageRecord {
  return isObject(value) && integer(value.sequence) && Array.isArray(value.page_indices) && value.page_indices.every(integer) &&
    source(value.object) && nullable(value.width, integer) && nullable(value.height, integer) &&
    nullable(value.filter, (v): v is string => typeof v === 'string') && nullable(value.color_space, (v): v is string => typeof v === 'string')
}
function summary(value: unknown): value is PdfDocumentSummary {
  if (!isObject(value) || !integer(value.raw_byte_length) || (value.version !== null && !version(value.version)) || !oneOf(value.xref_kind, xrefKinds) || !oneOf(value.encryption, encryptions)) return false
  return typeof value.binary_marker_present === 'boolean' && typeof value.eof_marker_present === 'boolean' &&
    [value.startxref_count, value.trailer_count, value.object_count, value.stream_count, value.object_stream_count, value.compressed_object_count, value.page_count, value.font_count, value.image_count, value.annotation_count, value.action_count, value.embedded_file_count].every(integer) &&
    typeof value.form_present === 'boolean' && typeof value.metadata_present === 'boolean' && typeof value.javascript_present === 'boolean' && typeof value.linearized === 'boolean'
}
function analysis(value: unknown): value is PdfAnalysis {
  return isObject(value) && summary(value.summary) && Array.isArray(value.objects) && value.objects.every((item) => isObject(item) && integer(item.object_number) && integer(item.generation) && typeof item.kind === 'string' && typeof item.has_stream === 'boolean' && typeof item.compressed_in_object_stream === 'boolean' && nullable(item.raw_byte_offset, integer)) &&
    Array.isArray(value.revisions) && value.revisions.every((item) => isObject(item) && integer(item.sequence) && nullable(item.startxref_offset, integer) && oneOf(item.xref_kind, xrefKinds) && nullable(item.prev_offset, integer) && typeof item.trailer_observed === 'boolean' && Array.isArray(item.updated_object_ids) && item.updated_object_ids.every(source)) &&
    metadata(value.metadata) && (value.provenance === undefined || (Array.isArray(value.provenance) && value.provenance.every(isProvenanceEvidence))) && Array.isArray(value.pages) && value.pages.every(page) && Array.isArray(value.images) && value.images.every(image) && Array.isArray(value.page_text) && value.page_text.every((item) => typeof item === 'string') &&
    Array.isArray(value.fonts) && value.fonts.every((item) => isObject(item) && integer(item.sequence) && Array.isArray(item.page_indices) && item.page_indices.every(integer) && typeof item.resource_name === 'string' && source(item.source) && nullable(item.subtype, (v): v is string => typeof v === 'string') && nullable(item.base_font, (v): v is string => typeof v === 'string') && nullable(item.encoding, (v): v is string => typeof v === 'string') && typeof item.to_unicode_present === 'boolean' && oneOf(item.mapping_status, statuses)) &&
    Array.isArray(value.actions) && value.actions.every((item) => isObject(item) && integer(item.sequence) && oneOf(item.kind, actionKinds) && nullable(item.target, (v): v is string => typeof v === 'string') && source(item.source) && typeof item.context === 'string') &&
    Array.isArray(value.annotations) && value.annotations.every((item) => isObject(item) && integer(item.sequence) && integer(item.page_index) && typeof item.subtype === 'string' && nullable(item.contents, (v): v is string => typeof v === 'string') && nullable(item.title, (v): v is string => typeof v === 'string') && nullable(item.uri, (v): v is string => typeof v === 'string') && nullable(item.action, (v): v is PdfActionKind => oneOf(v, actionKinds)) && nullable(item.flags, integer) && source(item.source)) &&
    Array.isArray(value.external_references) && value.external_references.every((item) => isObject(item) && integer(item.sequence) && typeof item.value === 'string' && oneOf(item.kind, externalKinds) && source(item.source)) &&
    Array.isArray(value.embedded_files) && value.embedded_files.every((item) => isObject(item) && integer(item.sequence) && nullable(item.declared_filename, (v): v is string => typeof v === 'string') && nullable(item.description, (v): v is string => typeof v === 'string') && nullable(item.mime_type, (v): v is string => typeof v === 'string') && nullable(item.size, integer) && source(item.object) && (item.stream_object === null || source(item.stream_object))) &&
    isObject(value.forms) && typeof value.forms.present === 'boolean' && integer(value.forms.field_count) && Array.isArray(value.forms.field_types) && value.forms.field_types.every((item) => typeof item === 'string') && nullable(value.forms.need_appearances, (v): v is boolean => typeof v === 'boolean') && typeof value.forms.xfa_present === 'boolean' &&
    Array.isArray(value.signatures) && value.signatures.every((item) => isObject(item) && integer(item.sequence) && source(item.source) && typeof item.byte_range_present === 'boolean' && typeof item.contents_present === 'boolean' && nullable(item.sub_filter, (v): v is string => typeof v === 'string') && nullable(item.reason, (v): v is string => typeof v === 'string') && nullable(item.name, (v): v is string => typeof v === 'string') && nullable(item.modification_date, (v): v is string => typeof v === 'string')) &&
    Array.isArray(value.text_segments) && value.text_segments.every((item) => isObject(item) && integer(item.sequence) && integer(item.page_index) && typeof item.text === 'string' && (item.font_reference === null || source(item.font_reference)) && (item.content_stream_object === null || source(item.content_stream_object)) && integer(item.operation_index) && (item.source_object === null || source(item.source_object)) && oneOf(item.extraction_status, statuses) && typeof item.explicit_non_painting === 'boolean') &&
    typeof value.document_text === 'string' && oneOf(value.text_extraction_status, statuses) && Array.isArray(value.findings) && value.findings.every((item) => isObject(item) && integer(item.sequence) && oneOf(item.kind, findingKinds) && oneOf(item.severity, ['informational', 'caution'] as const) && typeof item.description === 'string' && (item.source === null || source(item.source))) && isScanResult(value.scan)
}
function error(value: unknown): value is PdfAnalysisError {
  return isObject(value) && oneOf(value.code, ['empty_input', 'non_pdf_bytes', 'invalid_header', 'encrypted_pdf_unsupported', 'password_required', 'unsupported_encryption', 'malformed_structure', 'resource_limit_exceeded'] as const) && typeof value.message === 'string' && (value.resource_limit === null || oneOf(value.resource_limit, limits))
}

export function parsePdfAnalysisResponse(serialized: string): PdfAnalysisResponse {
  let parsed: unknown
  try { parsed = JSON.parse(serialized) as unknown } catch { throw new BridgeContractError('INVALID_JSON') }
  if (!isObject(parsed) || !((parsed.status === 'success' && analysis(parsed.analysis)) || (parsed.status === 'error' && error(parsed.error)))) throw new BridgeContractError('INVALID_SHAPE')
  return parsed as PdfAnalysisResponse
}
