import {
  BridgeContractError,
  isProvenanceEvidence,
  isScanResult,
  type ProvenanceEvidence,
  type ScanResult,
} from './contract.ts'

export type DocxPackageStatus =
  | 'valid_wordprocessing_package'
  | 'malformed_package'
  | 'unsupported_package'

export interface DocxPackageSummary {
  status: DocxPackageStatus
  raw_byte_length: number
  entry_count: number
  compressed_total: number
  uncompressed_total: number
  compression_ratio_milli: number
  zip64_present: boolean
  archive_offset: number
  duplicate_entry_names: number
  invalid_entry_names: number
}

export type DocxPartClassification =
  | 'content_types'
  | 'main_document'
  | 'relationships'
  | 'styles'
  | 'numbering'
  | 'settings'
  | 'theme'
  | 'font_table'
  | 'header'
  | 'footer'
  | 'footnotes'
  | 'endnotes'
  | 'comments'
  | 'metadata'
  | 'media'
  | 'embedded_object'
  | 'custom_xml'
  | 'active_content_related'
  | 'other'

export interface DocxPartRecord {
  sequence: number
  part_name: string
  content_type: string | null
  compressed_size: number
  uncompressed_size: number
  crc32: number
  crc_status: 'declared_only' | 'verified'
  compression_method: string
  classification: DocxPartClassification
  relationship_count: number
  encrypted: boolean
  directory: boolean
  symlink: boolean
  local_header_offset: number
  data_offset: number | null
  xml_encoding: 'utf_8' | 'utf_16_le' | 'utf_16_be' | null
}

export interface DocxByteRange {
  start: number
  end: number
}

export interface DocxSourceLocation {
  part_name: string
  decoded_xml_byte_range: DocxByteRange | null
  part_byte_range: DocxByteRange | null
  package_byte_range: DocxByteRange | null
}

export interface DocxContentTypeDeclaration {
  sequence: number
  kind: 'default' | 'override'
  extension: string | null
  part_name: string | null
  content_type: string
  source_location: DocxSourceLocation
}

export interface DocxRelationship {
  sequence: number
  relationship_part: string
  source_part: string | null
  id: string
  relationship_type: string
  target: string
  external: boolean
  resolved_internal_part: string | null
  source_location: DocxSourceLocation
}

export interface DocxMetadataProperty {
  sequence: number
  name: string
  value: string
  value_type: string | null
  source_location: DocxSourceLocation
}

export interface DocxMetadata {
  core_properties: DocxMetadataProperty[]
  app_properties: DocxMetadataProperty[]
  custom_properties: DocxMetadataProperty[]
}

export interface DocxDocumentSummary {
  main_document_present: boolean
  styles_present: boolean
  numbering_present: boolean
  paragraphs: number
  runs: number
  tables: number
  table_rows: number
  table_cells: number
  headers: number
  footers: number
  footnotes: number
  endnotes: number
  comments: number
  tracked_changes: number
  explicit_hidden_runs: number
  hyperlinks: number
  external_relationships: number
  media_parts: number
  embedded_object_parts: number
  custom_xml_parts: number
}

export type DocxTextChannel =
  | 'body'
  | 'header'
  | 'footer'
  | 'footnote'
  | 'endnote'
  | 'comment'
  | 'deleted_content'

export type DocxTextKind = 'text' | 'tab' | 'line_break' | 'no_break_hyphen' | 'soft_hyphen'
export type DocxRevisionKind = 'inserted' | 'deleted' | 'move_from' | 'move_to'

export interface DocxTextSegment {
  sequence: number
  text: string
  part_name: string
  channel: DocxTextChannel
  kind: DocxTextKind
  paragraph_index: number | null
  run_index: number | null
  table_index: number | null
  row_index: number | null
  cell_index: number | null
  xml_space_preserve: boolean
  explicit_hidden: boolean
  revision_kind: DocxRevisionKind | null
  source_location: DocxSourceLocation
}

export interface DocxPartTextChannel {
  part_name: string
  text: string
}

export interface DocxTextChannels {
  document_text: string
  headers: DocxPartTextChannel[]
  footers: DocxPartTextChannel[]
  footnotes: DocxPartTextChannel[]
  endnotes: DocxPartTextChannel[]
  comments: DocxPartTextChannel[]
  deleted_content: string
}

export interface DocxComment {
  sequence: number
  id: string | null
  declared_author: string | null
  initials: string | null
  declared_date: string | null
  text: string
  source_location: DocxSourceLocation
}

export interface DocxRevision {
  sequence: number
  kind: DocxRevisionKind
  id: string | null
  declared_author: string | null
  declared_date: string | null
  text: string
  source_location: DocxSourceLocation
}

export interface DocxHyperlink {
  sequence: number
  part_name: string
  display_text: string
  relationship_id: string | null
  target: string | null
  external: boolean
  source_location: DocxSourceLocation
}

export type DocxFindingKind =
  | 'unsafe_entry_name'
  | 'ambiguous_entry_name'
  | 'prepended_zip_data_present'
  | 'symlink_entry_present'
  | 'external_relationship_present'
  | 'embedded_object_present'
  | 'active_content_related_part_present'
  | 'unexpected_package_part'
  | 'tracked_changes_present'
  | 'comments_present'
  | 'explicit_hidden_text_present'
  | 'custom_xml_present'
  | 'metadata_present'
  | 'malformed_relationship'

export interface DocxFinding {
  sequence: number
  kind: DocxFindingKind
  severity: 'informational' | 'caution'
  part_name: string | null
  relationship_sequence: number | null
  description: string
  source_location: DocxSourceLocation | null
}

export interface DocxAnalysis {
  package: DocxPackageSummary
  parts: DocxPartRecord[]
  content_types: DocxContentTypeDeclaration[]
  relationships: DocxRelationship[]
  metadata: DocxMetadata
  provenance?: ProvenanceEvidence[]
  summary: DocxDocumentSummary
  text_segments: DocxTextSegment[]
  comments: DocxComment[]
  revisions: DocxRevision[]
  hyperlinks: DocxHyperlink[]
  findings: DocxFinding[]
  text_channels: DocxTextChannels
  scan: ScanResult
}

export type DocxResourceLimit =
  | 'source_bytes'
  | 'entry_count'
  | 'total_uncompressed_bytes'
  | 'part_bytes'
  | 'compression_ratio'
  | 'entry_name_bytes'
  | 'xml_part_bytes'
  | 'relationships'
  | 'text_segments'
  | 'xml_nodes'
  | 'xml_depth'

export type DocxAnalysisErrorCode =
  | 'invalid_zip'
  | 'duplicate_entry_names'
  | 'encrypted_entry'
  | 'unsupported_compression'
  | 'resource_limit_exceeded'
  | 'missing_content_types'
  | 'missing_root_relationships'
  | 'missing_main_document'
  | 'malformed_xml'
  | 'malformed_relationships'
  | 'unsupported_package'
  | 'part_read_failure'

export interface DocxAnalysisError {
  code: DocxAnalysisErrorCode
  message: string
  resource_limit: DocxResourceLimit | null
}

export type DocxAnalysisResponse =
  | { status: 'success'; analysis: DocxAnalysis }
  | { status: 'error'; error: DocxAnalysisError }

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function integer(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}

function stringOrNull(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}

function integerOrNull(value: unknown): value is number | null {
  return value === null || integer(value)
}

function isOneOf<T extends string>(value: unknown, variants: readonly T[]): value is T {
  return typeof value === 'string' && variants.includes(value as T)
}

function isByteRange(value: unknown): value is DocxByteRange {
  return isObject(value) && integer(value.start) && integer(value.end) && value.end >= value.start
}

function isLocation(value: unknown): value is DocxSourceLocation {
  return (
    isObject(value) &&
    typeof value.part_name === 'string' &&
    (value.decoded_xml_byte_range === null || isByteRange(value.decoded_xml_byte_range)) &&
    (value.part_byte_range === null || isByteRange(value.part_byte_range)) &&
    (value.package_byte_range === null || isByteRange(value.package_byte_range))
  )
}

const packageStatuses = [
  'valid_wordprocessing_package',
  'malformed_package',
  'unsupported_package',
] as const
const classifications = [
  'content_types', 'main_document', 'relationships', 'styles', 'numbering', 'settings', 'theme',
  'font_table', 'header', 'footer', 'footnotes', 'endnotes', 'comments', 'metadata', 'media',
  'embedded_object', 'custom_xml', 'active_content_related', 'other',
] as const
const revisionKinds = ['inserted', 'deleted', 'move_from', 'move_to'] as const
const channelKinds = ['body', 'header', 'footer', 'footnote', 'endnote', 'comment', 'deleted_content'] as const
const textKinds = ['text', 'tab', 'line_break', 'no_break_hyphen', 'soft_hyphen'] as const
const findingKinds = [
  'unsafe_entry_name', 'ambiguous_entry_name', 'prepended_zip_data_present',
  'symlink_entry_present', 'external_relationship_present', 'embedded_object_present',
  'active_content_related_part_present', 'unexpected_package_part', 'tracked_changes_present',
  'comments_present', 'explicit_hidden_text_present', 'custom_xml_present', 'metadata_present',
  'malformed_relationship',
] as const
const errorCodes = [
  'invalid_zip', 'duplicate_entry_names', 'encrypted_entry', 'unsupported_compression',
  'resource_limit_exceeded', 'missing_content_types', 'missing_root_relationships',
  'missing_main_document', 'malformed_xml', 'malformed_relationships', 'unsupported_package',
  'part_read_failure',
] as const
const resourceLimits = [
  'source_bytes', 'entry_count', 'total_uncompressed_bytes', 'part_bytes', 'compression_ratio',
  'entry_name_bytes', 'xml_part_bytes', 'relationships', 'text_segments', 'xml_nodes', 'xml_depth',
] as const

function isPackage(value: unknown): value is DocxPackageSummary {
  if (!isObject(value) || !isOneOf(value.status, packageStatuses)) return false
  const numbers = [value.raw_byte_length, value.entry_count, value.compressed_total,
    value.uncompressed_total, value.compression_ratio_milli, value.archive_offset,
    value.duplicate_entry_names, value.invalid_entry_names]
  return numbers.every(integer) && typeof value.zip64_present === 'boolean'
}

function isPart(value: unknown): value is DocxPartRecord {
  return (
    isObject(value) && integer(value.sequence) && typeof value.part_name === 'string' &&
    stringOrNull(value.content_type) && integer(value.compressed_size) &&
    integer(value.uncompressed_size) && integer(value.crc32) &&
    isOneOf(value.crc_status, ['declared_only', 'verified'] as const) &&
    typeof value.compression_method === 'string' && isOneOf(value.classification, classifications) &&
    integer(value.relationship_count) && typeof value.encrypted === 'boolean' &&
    typeof value.directory === 'boolean' && typeof value.symlink === 'boolean' &&
    integer(value.local_header_offset) && integerOrNull(value.data_offset) &&
    (value.xml_encoding === null || isOneOf(value.xml_encoding, ['utf_8', 'utf_16_le', 'utf_16_be'] as const))
  )
}

function isContentType(value: unknown): value is DocxContentTypeDeclaration {
  return isObject(value) && integer(value.sequence) && isOneOf(value.kind, ['default', 'override'] as const) &&
    stringOrNull(value.extension) && stringOrNull(value.part_name) &&
    typeof value.content_type === 'string' && isLocation(value.source_location)
}

function isRelationship(value: unknown): value is DocxRelationship {
  return isObject(value) && integer(value.sequence) && typeof value.relationship_part === 'string' &&
    stringOrNull(value.source_part) && typeof value.id === 'string' &&
    typeof value.relationship_type === 'string' && typeof value.target === 'string' &&
    typeof value.external === 'boolean' && stringOrNull(value.resolved_internal_part) &&
    isLocation(value.source_location)
}

function isMetadataProperty(value: unknown): value is DocxMetadataProperty {
  return isObject(value) && integer(value.sequence) && typeof value.name === 'string' &&
    typeof value.value === 'string' && stringOrNull(value.value_type) && isLocation(value.source_location)
}

function isMetadata(value: unknown): value is DocxMetadata {
  return isObject(value) && Array.isArray(value.core_properties) &&
    value.core_properties.every(isMetadataProperty) && Array.isArray(value.app_properties) &&
    value.app_properties.every(isMetadataProperty) && Array.isArray(value.custom_properties) &&
    value.custom_properties.every(isMetadataProperty)
}

function isSummary(value: unknown): value is DocxDocumentSummary {
  if (!isObject(value) || typeof value.main_document_present !== 'boolean' ||
    typeof value.styles_present !== 'boolean' || typeof value.numbering_present !== 'boolean') return false
  return [value.paragraphs, value.runs, value.tables, value.table_rows, value.table_cells,
    value.headers, value.footers, value.footnotes, value.endnotes, value.comments,
    value.tracked_changes, value.explicit_hidden_runs, value.hyperlinks,
    value.external_relationships, value.media_parts, value.embedded_object_parts,
    value.custom_xml_parts].every(integer)
}

function isTextSegment(value: unknown): value is DocxTextSegment {
  return isObject(value) && integer(value.sequence) && typeof value.text === 'string' &&
    typeof value.part_name === 'string' && isOneOf(value.channel, channelKinds) &&
    isOneOf(value.kind, textKinds) && integerOrNull(value.paragraph_index) &&
    integerOrNull(value.run_index) && integerOrNull(value.table_index) &&
    integerOrNull(value.row_index) && integerOrNull(value.cell_index) &&
    typeof value.xml_space_preserve === 'boolean' && typeof value.explicit_hidden === 'boolean' &&
    (value.revision_kind === null || isOneOf(value.revision_kind, revisionKinds)) &&
    isLocation(value.source_location)
}

function isPartChannel(value: unknown): value is DocxPartTextChannel {
  return isObject(value) && typeof value.part_name === 'string' && typeof value.text === 'string'
}

function isChannels(value: unknown): value is DocxTextChannels {
  return isObject(value) && typeof value.document_text === 'string' &&
    Array.isArray(value.headers) && value.headers.every(isPartChannel) &&
    Array.isArray(value.footers) && value.footers.every(isPartChannel) &&
    Array.isArray(value.footnotes) && value.footnotes.every(isPartChannel) &&
    Array.isArray(value.endnotes) && value.endnotes.every(isPartChannel) &&
    Array.isArray(value.comments) && value.comments.every(isPartChannel) &&
    typeof value.deleted_content === 'string'
}

function isComment(value: unknown): value is DocxComment {
  return isObject(value) && integer(value.sequence) && stringOrNull(value.id) &&
    stringOrNull(value.declared_author) && stringOrNull(value.initials) &&
    stringOrNull(value.declared_date) && typeof value.text === 'string' && isLocation(value.source_location)
}

function isRevision(value: unknown): value is DocxRevision {
  return isObject(value) && integer(value.sequence) && isOneOf(value.kind, revisionKinds) &&
    stringOrNull(value.id) && stringOrNull(value.declared_author) &&
    stringOrNull(value.declared_date) && typeof value.text === 'string' && isLocation(value.source_location)
}

function isHyperlink(value: unknown): value is DocxHyperlink {
  return isObject(value) && integer(value.sequence) && typeof value.part_name === 'string' &&
    typeof value.display_text === 'string' && stringOrNull(value.relationship_id) &&
    stringOrNull(value.target) && typeof value.external === 'boolean' && isLocation(value.source_location)
}

function isFinding(value: unknown): value is DocxFinding {
  return isObject(value) && integer(value.sequence) && isOneOf(value.kind, findingKinds) &&
    isOneOf(value.severity, ['informational', 'caution'] as const) && stringOrNull(value.part_name) &&
    integerOrNull(value.relationship_sequence) && typeof value.description === 'string' &&
    (value.source_location === null || isLocation(value.source_location))
}

function isAnalysis(value: unknown): value is DocxAnalysis {
  return isObject(value) && isPackage(value.package) && Array.isArray(value.parts) &&
    value.parts.every(isPart) && Array.isArray(value.content_types) &&
    value.content_types.every(isContentType) && Array.isArray(value.relationships) &&
    value.relationships.every(isRelationship) && isMetadata(value.metadata) &&
    (value.provenance === undefined || (Array.isArray(value.provenance) && value.provenance.every(isProvenanceEvidence))) &&
    isSummary(value.summary) && Array.isArray(value.text_segments) &&
    value.text_segments.every(isTextSegment) && Array.isArray(value.comments) &&
    value.comments.every(isComment) && Array.isArray(value.revisions) &&
    value.revisions.every(isRevision) && Array.isArray(value.hyperlinks) &&
    value.hyperlinks.every(isHyperlink) && Array.isArray(value.findings) &&
    value.findings.every(isFinding) && isChannels(value.text_channels) && isScanResult(value.scan)
}

function isError(value: unknown): value is DocxAnalysisError {
  return isObject(value) && isOneOf(value.code, errorCodes) && typeof value.message === 'string' &&
    (value.resource_limit === null || isOneOf(value.resource_limit, resourceLimits))
}

export function parseDocxAnalysisResponse(serialized: string): DocxAnalysisResponse {
  let parsed: unknown
  try {
    parsed = JSON.parse(serialized) as unknown
  } catch {
    throw new BridgeContractError('INVALID_JSON')
  }

  if (!isObject(parsed) ||
    !((parsed.status === 'success' && isAnalysis(parsed.analysis)) ||
      (parsed.status === 'error' && isError(parsed.error)))) {
    throw new BridgeContractError('INVALID_SHAPE')
  }

  return parsed as DocxAnalysisResponse
}
