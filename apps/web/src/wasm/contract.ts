export interface CharacterRecord {
  code_point_index: number
  utf16_offset: number
  utf8_byte_offset: number
  character: string
  code_point: number
  code_point_label: string
  utf8_bytes: number[]
  unicode_category: string
  script: string
  is_whitespace: boolean
  is_control: boolean
  is_format: boolean
  is_invisible: boolean
  is_unusual_whitespace: boolean
  is_bidi_control: boolean
  is_variation_selector: boolean
}

export interface Finding {
  kind: string
  severity: string
  code_point_index: number
  utf16_offset: number
  utf8_byte_offset: number
  code_point: number
  description: string
}

export interface NormalizationInspection {
  nfc_would_change: boolean
  nfd_would_change: boolean
  nfkc_would_change: boolean
  nfkd_would_change: boolean
}

export interface ScanResult {
  characters: CharacterRecord[]
  findings: Finding[]
  normalization: NormalizationInspection
}

export type ProvenanceEvidenceKind =
  | 'metadata_declaration'
  | 'generator_declaration'
  | 'creator_declaration'
  | 'author_declaration'
  | 'producer_declaration'
  | 'canonical_declaration'
  | 'encoding_declaration'
  | 'clipboard_representation'
  | 'attribution'

export type ProvenanceEvidenceSource =
  | 'html_metadata'
  | 'html_generator'
  | 'html_canonical_link'
  | 'html_encoding'
  | 'docx_core_properties'
  | 'docx_app_properties'
  | 'docx_custom_properties'
  | 'pdf_info'
  | 'pdf_xmp'
  | 'clipboard_plain_text'
  | 'clipboard_html'
  | 'clipboard_other_mime'

export type ProvenanceEvidenceLevel = 'observation' | 'claim' | 'verification' | 'attribution'
export type ProvenanceVerificationState =
  | 'observed'
  | 'declared'
  | 'verified'
  | 'not_verified'
  | 'unsupported'
  | 'conflict'

export interface ProvenanceEvidence {
  kind: ProvenanceEvidenceKind
  source: ProvenanceEvidenceSource
  subject: string
  value: string
  level: ProvenanceEvidenceLevel
  verification: ProvenanceVerificationState
}

export interface ScanSummary {
  characters: number
  invisible: number
  unusualWhitespace: number
  bidiControls: number
}

export type TextSourceKind = 'plain_text' | 'markdown'

export type SourceEncoding = 'utf_8' | 'utf_16_le' | 'utf_16_be'

export type EncodingEvidence = 'empty_input' | 'strict_utf_8' | 'byte_order_mark'

export type BomKind = 'none' | 'utf_8' | 'utf_16_le' | 'utf_16_be'

export type DecodeStatus = 'decoded'

export interface SourceCharacterMapping {
  code_point_index: number
  source_byte_range: TextRange
  decoded_utf8_byte_offset: number
  utf16_offset: number
}

export interface TextDecodeResult {
  encoding: SourceEncoding
  encoding_evidence: EncodingEvidence
  bom: BomKind
  status: DecodeStatus
  decoded_text: string
  source_mappings: SourceCharacterMapping[]
}

export interface TextSourceAnalysis {
  raw_byte_length: number
  source_kind: TextSourceKind
  decode: TextDecodeResult
  scan: ScanResult
}

export type TextIngestionErrorCode =
  | 'unsupported_file_type'
  | 'unsupported_encoding'
  | 'ambiguous_encoding'
  | 'invalid_utf8'
  | 'invalid_utf16'

export interface TextIngestionError {
  code: TextIngestionErrorCode
  message: string
}

export type TextSourceAnalysisResponse =
  | { status: 'success'; analysis: TextSourceAnalysis }
  | { status: 'error'; error: TextIngestionError }

export type HtmlEncodingDeclarationKind = 'meta_charset' | 'http_equiv_content_type'

export interface HtmlSourceRange {
  decoded_utf8_byte_range: TextRange
  raw_source_byte_range: TextRange | null
}

export interface HtmlEncodingDeclaration {
  sequence: number
  kind: HtmlEncodingDeclarationKind
  declared_label: string
  normalized_label: string
  source_range: HtmlSourceRange
}

export interface HtmlEncodingEvidence {
  declarations: HtmlEncodingDeclaration[]
  bom_precedence_applied: boolean
  declaration_conflict: boolean
}

export interface HtmlDoctypeRecord {
  name: string
  public_identifier: string | null
  system_identifier: string | null
  force_quirks: boolean
  source_range: HtmlSourceRange
}

export interface HtmlDocumentSummary {
  doctype: HtmlDoctypeRecord | null
  quirks_mode: string
  parsed_elements: number
  source_start_tags: number
  attributes: number
  text_segments: number
  comments: number
  script_blocks: number
  style_blocks: number
  noscript_blocks: number
  template_blocks: number
  external_references: number
  tokenizer_recoveries: number
  tree_builder_recoveries: number
  html_present: boolean
  head_present: boolean
  body_present: boolean
  title_present: boolean
  meta_elements: number
  link_elements: number
}

export interface HtmlMetaDeclaration {
  sequence: number
  charset: string | null
  name: string | null
  property: string | null
  content: string | null
  source_range: HtmlSourceRange
}

export interface HtmlMetadata {
  title: string | null
  lang: string | null
  dir: string | null
  meta_declarations: HtmlMetaDeclaration[]
  canonical_links: string[]
  base_hrefs: string[]
}

export interface HtmlAttributeRecord {
  name: string
  value: string
  source_range: HtmlSourceRange
}

export interface HtmlElementRecord {
  sequence: number
  name: string
  attributes: HtmlAttributeRecord[]
  self_closing: boolean
  source_range: HtmlSourceRange
}

export type HtmlTextClassification =
  | 'visible_content_candidate'
  | 'title'
  | 'metadata'
  | 'script_content'
  | 'style_content'
  | 'template_content'
  | 'noscript_content'

export interface HtmlTextSegment {
  sequence: number
  text: string
  context_element: string
  classification: HtmlTextClassification
  source_text: string | null
  source_range: HtmlSourceRange | null
}

export interface HtmlCommentRecord {
  sequence: number
  text: string
  source_range: HtmlSourceRange
}

export type HtmlReferenceKind = 'relative' | 'absolute' | 'data_url' | 'fragment' | 'other'

export interface HtmlExternalReference {
  sequence: number
  element: string
  attribute: string
  value: string
  kind: HtmlReferenceKind
  source_range: HtmlSourceRange
}

export type HtmlFindingKind =
  | 'active_script_present'
  | 'inline_event_handler_present'
  | 'external_reference_present'
  | 'explicit_hidden_marker'
  | 'encoding_declaration_conflict'
  | 'parser_recovery_observed'

export type HtmlFindingSeverity = 'informational' | 'caution'

export interface HtmlFinding {
  sequence: number
  kind: HtmlFindingKind
  severity: HtmlFindingSeverity
  element_sequence: number | null
  attribute: string | null
  description: string
  source_range: HtmlSourceRange | null
}

export type HtmlParserLayer = 'tokenizer' | 'tree_builder'

export interface HtmlParserRecovery {
  sequence: number
  layer: HtmlParserLayer
  message: string
  source_range: HtmlSourceRange | null
}

export interface HtmlTextChannels {
  document_text: string
  metadata_text: string
  active_content_text: string
  style_text: string
  template_text: string
  noscript_text: string
}

export interface HtmlAnalysis {
  raw_byte_length: number
  decode: TextDecodeResult
  encoding: HtmlEncodingEvidence
  summary: HtmlDocumentSummary
  metadata: HtmlMetadata
  provenance?: ProvenanceEvidence[]
  elements: HtmlElementRecord[]
  text_segments: HtmlTextSegment[]
  comments: HtmlCommentRecord[]
  external_references: HtmlExternalReference[]
  findings: HtmlFinding[]
  parser_recoveries: HtmlParserRecovery[]
  text_channels: HtmlTextChannels
  scan: ScanResult
}

export type HtmlAnalysisErrorCode =
  | Exclude<TextIngestionErrorCode, 'unsupported_file_type'>
  | 'unsupported_declared_encoding'
  | 'resource_limit_exceeded'
  | 'html_parser_failure'

export type HtmlResourceLimit =
  | 'source_bytes'
  | 'tokens'
  | 'elements'
  | 'attributes'
  | 'nesting_depth'
  | 'parse_recoveries'
  | 'comments'
  | 'attribute_value_bytes'
  | 'comment_bytes'
  | 'text_token_bytes'
  | 'parsed_nodes'

export interface HtmlAnalysisError {
  code: HtmlAnalysisErrorCode
  message: string
  resource_limit: HtmlResourceLimit | null
}

export type HtmlAnalysisResponse =
  | { status: 'success'; analysis: HtmlAnalysis }
  | { status: 'error'; error: HtmlAnalysisError }

export type CleanProfile =
  | 'safe_clean'
  | 'canonical_clean'
  | 'pure_plain_text'
  | 'document_clean'
  | 'developer_clean'

export interface CleanProfileDefinition {
  id: CleanProfile
  label: string
  description: string
  supported: boolean
  active_rule_ids: string[]
}

export interface TextRange {
  start: number
  end: number
}

export type TransformationOperation =
  | 'normalize_nfc'
  | 'canonicalize_line_ending'
  | 'remove_leading_byte_order_mark'
  | 'replace_non_breaking_space'

export interface Transformation {
  sequence: number
  rule_id: string
  rule_version: string
  operation: TransformationOperation
  original_code_point_range: TextRange
  original_utf8_byte_range: TextRange
  replacement: string
  reason: string
  before: string
  after: string
}

export interface TransformationManifest {
  tcs_version: string
  profile: CleanProfile
  transformations: Transformation[]
}

export interface TechnicalDiff {
  changed: boolean
  original_utf8_bytes: number
  clean_utf8_bytes: number
  original_code_points: number
  clean_code_points: number
  transformation_count: number
}

export type ContentPreservationStatus = 'preserved' | 'needs_review' | 'unknown'

export interface ContentPreservationDiff {
  status: ContentPreservationStatus
  reason: string
}

export interface CleanScanSummary {
  characters: number
  findings: number
  invisible: number
  unusual_whitespace: number
  bidi_controls: number
  variation_selectors: number
}

export interface VerificationChecks {
  manifest_reproduces_clean_text: boolean
  rules_allowed_by_profile: boolean
  no_undeclared_transformations: boolean
  clean_rescan_completed: boolean
  expected_findings_reassessed: boolean
  preserved_findings_visible: boolean
  deterministic: boolean
  idempotent: boolean
}

export type VerificationStatus =
  | 'verified'
  | 'verified_with_remaining_findings'
  | 'needs_review'
  | 'failed'

export interface VerificationReport {
  original_scan_summary: CleanScanSummary
  clean_scan_summary: CleanScanSummary
  applied_transformations: number
  remaining_findings: Finding[]
  content_preservation_status: ContentPreservationStatus
  verification_status: VerificationStatus
  checks: VerificationChecks
}

export interface CleanResult {
  original_text: string
  clean_text: string
  profile: CleanProfile
  tcs_version: string
  manifest: TransformationManifest
  technical_diff: TechnicalDiff
  content_preservation_diff: ContentPreservationDiff
  verification: VerificationReport
}

export type BridgeContractErrorCode = 'INVALID_JSON' | 'INVALID_SHAPE'

export class BridgeContractError extends Error {
  readonly code: BridgeContractErrorCode

  constructor(code: BridgeContractErrorCode) {
    super('The TextScope bridge returned an invalid response.')
    this.name = 'BridgeContractError'
    this.code = code
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}

function hasString(record: Record<string, unknown>, field: string): boolean {
  return typeof record[field] === 'string'
}

function hasBoolean(record: Record<string, unknown>, field: string): boolean {
  return typeof record[field] === 'boolean'
}

function isCharacterRecord(value: unknown): value is CharacterRecord {
  if (!isObject(value)) {
    return false
  }

  return (
    isSafeInteger(value.code_point_index) &&
    isSafeInteger(value.utf16_offset) &&
    isSafeInteger(value.utf8_byte_offset) &&
    hasString(value, 'character') &&
    isSafeInteger(value.code_point) &&
    hasString(value, 'code_point_label') &&
    Array.isArray(value.utf8_bytes) &&
    value.utf8_bytes.every(isSafeInteger) &&
    hasString(value, 'unicode_category') &&
    hasString(value, 'script') &&
    hasBoolean(value, 'is_whitespace') &&
    hasBoolean(value, 'is_control') &&
    hasBoolean(value, 'is_format') &&
    hasBoolean(value, 'is_invisible') &&
    hasBoolean(value, 'is_unusual_whitespace') &&
    hasBoolean(value, 'is_bidi_control') &&
    hasBoolean(value, 'is_variation_selector')
  )
}

function isFinding(value: unknown): value is Finding {
  if (!isObject(value)) {
    return false
  }

  return (
    hasString(value, 'kind') &&
    hasString(value, 'severity') &&
    isSafeInteger(value.code_point_index) &&
    isSafeInteger(value.utf16_offset) &&
    isSafeInteger(value.utf8_byte_offset) &&
    isSafeInteger(value.code_point) &&
    hasString(value, 'description')
  )
}

function isCleanProfile(value: unknown): value is CleanProfile {
  return (
    value === 'safe_clean' ||
    value === 'canonical_clean' ||
    value === 'pure_plain_text' ||
    value === 'document_clean' ||
    value === 'developer_clean'
  )
}

function isTextSourceKind(value: unknown): value is TextSourceKind {
  return value === 'plain_text' || value === 'markdown'
}

function isSourceEncoding(value: unknown): value is SourceEncoding {
  return value === 'utf_8' || value === 'utf_16_le' || value === 'utf_16_be'
}

function isEncodingEvidence(value: unknown): value is EncodingEvidence {
  return value === 'empty_input' || value === 'strict_utf_8' || value === 'byte_order_mark'
}

function isBomKind(value: unknown): value is BomKind {
  return value === 'none' || value === 'utf_8' || value === 'utf_16_le' || value === 'utf_16_be'
}

function isTextIngestionErrorCode(value: unknown): value is TextIngestionErrorCode {
  return (
    value === 'unsupported_file_type' ||
    value === 'unsupported_encoding' ||
    value === 'ambiguous_encoding' ||
    value === 'invalid_utf8' ||
    value === 'invalid_utf16'
  )
}

function isTransformationOperation(value: unknown): value is TransformationOperation {
  return (
    value === 'normalize_nfc' ||
    value === 'canonicalize_line_ending' ||
    value === 'remove_leading_byte_order_mark' ||
    value === 'replace_non_breaking_space'
  )
}

function isContentPreservationStatus(value: unknown): value is ContentPreservationStatus {
  return value === 'preserved' || value === 'needs_review' || value === 'unknown'
}

function isVerificationStatus(value: unknown): value is VerificationStatus {
  return (
    value === 'verified' ||
    value === 'verified_with_remaining_findings' ||
    value === 'needs_review' ||
    value === 'failed'
  )
}

function isTextRange(value: unknown): value is TextRange {
  return (
    isObject(value) &&
    isSafeInteger(value.start) &&
    isSafeInteger(value.end) &&
    value.start <= value.end
  )
}

function isTransformation(value: unknown): value is Transformation {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    hasString(value, 'rule_id') &&
    hasString(value, 'rule_version') &&
    isTransformationOperation(value.operation) &&
    isTextRange(value.original_code_point_range) &&
    isTextRange(value.original_utf8_byte_range) &&
    hasString(value, 'replacement') &&
    hasString(value, 'reason') &&
    hasString(value, 'before') &&
    hasString(value, 'after')
  )
}

function isTransformationManifest(value: unknown): value is TransformationManifest {
  return (
    isObject(value) &&
    hasString(value, 'tcs_version') &&
    isCleanProfile(value.profile) &&
    Array.isArray(value.transformations) &&
    value.transformations.every(isTransformation)
  )
}

function isTechnicalDiff(value: unknown): value is TechnicalDiff {
  return (
    isObject(value) &&
    hasBoolean(value, 'changed') &&
    isSafeInteger(value.original_utf8_bytes) &&
    isSafeInteger(value.clean_utf8_bytes) &&
    isSafeInteger(value.original_code_points) &&
    isSafeInteger(value.clean_code_points) &&
    isSafeInteger(value.transformation_count)
  )
}

function isContentPreservationDiff(value: unknown): value is ContentPreservationDiff {
  return (
    isObject(value) &&
    isContentPreservationStatus(value.status) &&
    hasString(value, 'reason')
  )
}

function isCleanScanSummary(value: unknown): value is CleanScanSummary {
  return (
    isObject(value) &&
    isSafeInteger(value.characters) &&
    isSafeInteger(value.findings) &&
    isSafeInteger(value.invisible) &&
    isSafeInteger(value.unusual_whitespace) &&
    isSafeInteger(value.bidi_controls) &&
    isSafeInteger(value.variation_selectors)
  )
}

function isVerificationChecks(value: unknown): value is VerificationChecks {
  return (
    isObject(value) &&
    hasBoolean(value, 'manifest_reproduces_clean_text') &&
    hasBoolean(value, 'rules_allowed_by_profile') &&
    hasBoolean(value, 'no_undeclared_transformations') &&
    hasBoolean(value, 'clean_rescan_completed') &&
    hasBoolean(value, 'expected_findings_reassessed') &&
    hasBoolean(value, 'preserved_findings_visible') &&
    hasBoolean(value, 'deterministic') &&
    hasBoolean(value, 'idempotent')
  )
}

function isVerificationReport(value: unknown): value is VerificationReport {
  return (
    isObject(value) &&
    isCleanScanSummary(value.original_scan_summary) &&
    isCleanScanSummary(value.clean_scan_summary) &&
    isSafeInteger(value.applied_transformations) &&
    Array.isArray(value.remaining_findings) &&
    value.remaining_findings.every(isFinding) &&
    isContentPreservationStatus(value.content_preservation_status) &&
    isVerificationStatus(value.verification_status) &&
    isVerificationChecks(value.checks)
  )
}

function isCleanResult(value: unknown): value is CleanResult {
  return (
    isObject(value) &&
    hasString(value, 'original_text') &&
    hasString(value, 'clean_text') &&
    isCleanProfile(value.profile) &&
    hasString(value, 'tcs_version') &&
    isTransformationManifest(value.manifest) &&
    isTechnicalDiff(value.technical_diff) &&
    isContentPreservationDiff(value.content_preservation_diff) &&
    isVerificationReport(value.verification)
  )
}

function isCleanProfileDefinition(value: unknown): value is CleanProfileDefinition {
  return (
    isObject(value) &&
    isCleanProfile(value.id) &&
    hasString(value, 'label') &&
    hasString(value, 'description') &&
    hasBoolean(value, 'supported') &&
    Array.isArray(value.active_rule_ids) &&
    value.active_rule_ids.every((ruleId) => typeof ruleId === 'string')
  )
}

function isNormalizationInspection(value: unknown): value is NormalizationInspection {
  if (!isObject(value)) {
    return false
  }

  return (
    hasBoolean(value, 'nfc_would_change') &&
    hasBoolean(value, 'nfd_would_change') &&
    hasBoolean(value, 'nfkc_would_change') &&
    hasBoolean(value, 'nfkd_would_change')
  )
}

export function isScanResult(value: unknown): value is ScanResult {
  return (
    isObject(value) &&
    Array.isArray(value.characters) &&
    value.characters.every(isCharacterRecord) &&
    Array.isArray(value.findings) &&
    value.findings.every(isFinding) &&
    isNormalizationInspection(value.normalization)
  )
}

function isSourceCharacterMapping(value: unknown): value is SourceCharacterMapping {
  return (
    isObject(value) &&
    isSafeInteger(value.code_point_index) &&
    isTextRange(value.source_byte_range) &&
    isSafeInteger(value.decoded_utf8_byte_offset) &&
    isSafeInteger(value.utf16_offset)
  )
}

function isTextDecodeResult(value: unknown): value is TextDecodeResult {
  return (
    isObject(value) &&
    isSourceEncoding(value.encoding) &&
    isEncodingEvidence(value.encoding_evidence) &&
    isBomKind(value.bom) &&
    value.status === 'decoded' &&
    hasString(value, 'decoded_text') &&
    Array.isArray(value.source_mappings) &&
    value.source_mappings.every(isSourceCharacterMapping)
  )
}

function isTextSourceAnalysis(value: unknown): value is TextSourceAnalysis {
  return (
    isObject(value) &&
    isSafeInteger(value.raw_byte_length) &&
    isTextSourceKind(value.source_kind) &&
    isTextDecodeResult(value.decode) &&
    isScanResult(value.scan) &&
    value.decode.source_mappings.length === value.scan.characters.length
  )
}

function isTextIngestionError(value: unknown): value is TextIngestionError {
  return (
    isObject(value) && isTextIngestionErrorCode(value.code) && hasString(value, 'message')
  )
}

function isTextSourceAnalysisResponse(value: unknown): value is TextSourceAnalysisResponse {
  if (!isObject(value)) {
    return false
  }

  return (
    (value.status === 'success' && isTextSourceAnalysis(value.analysis)) ||
    (value.status === 'error' && isTextIngestionError(value.error))
  )
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === 'string'
}

const provenanceKinds = [
  'metadata_declaration', 'generator_declaration', 'creator_declaration',
  'author_declaration', 'producer_declaration', 'canonical_declaration',
  'encoding_declaration', 'clipboard_representation', 'attribution',
] as const
const provenanceSources = [
  'html_metadata', 'html_generator', 'html_canonical_link', 'html_encoding',
  'docx_core_properties', 'docx_app_properties', 'docx_custom_properties',
  'pdf_info', 'pdf_xmp', 'clipboard_plain_text', 'clipboard_html',
  'clipboard_other_mime',
] as const
const provenanceLevels = ['observation', 'claim', 'verification', 'attribution'] as const
const provenanceStates = ['observed', 'declared', 'verified', 'not_verified', 'unsupported', 'conflict'] as const

function isOneOf<T extends string>(value: unknown, values: readonly T[]): value is T {
  return typeof value === 'string' && values.includes(value as T)
}

export function isProvenanceEvidence(value: unknown): value is ProvenanceEvidence {
  return (
    isObject(value) &&
    isOneOf(value.kind, provenanceKinds) &&
    isOneOf(value.source, provenanceSources) &&
    typeof value.subject === 'string' &&
    typeof value.value === 'string' &&
    isOneOf(value.level, provenanceLevels) &&
    isOneOf(value.verification, provenanceStates)
  )
}

function isHtmlSourceRange(value: unknown): value is HtmlSourceRange {
  return (
    isObject(value) &&
    isTextRange(value.decoded_utf8_byte_range) &&
    (value.raw_source_byte_range === null || isTextRange(value.raw_source_byte_range))
  )
}

function isHtmlEncodingDeclarationKind(value: unknown): value is HtmlEncodingDeclarationKind {
  return value === 'meta_charset' || value === 'http_equiv_content_type'
}

function isHtmlEncodingDeclaration(value: unknown): value is HtmlEncodingDeclaration {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    isHtmlEncodingDeclarationKind(value.kind) &&
    hasString(value, 'declared_label') &&
    hasString(value, 'normalized_label') &&
    isHtmlSourceRange(value.source_range)
  )
}

function isHtmlEncodingEvidence(value: unknown): value is HtmlEncodingEvidence {
  return (
    isObject(value) &&
    Array.isArray(value.declarations) &&
    value.declarations.every(isHtmlEncodingDeclaration) &&
    hasBoolean(value, 'bom_precedence_applied') &&
    hasBoolean(value, 'declaration_conflict')
  )
}

function isHtmlDoctypeRecord(value: unknown): value is HtmlDoctypeRecord {
  return (
    isObject(value) &&
    hasString(value, 'name') &&
    isNullableString(value.public_identifier) &&
    isNullableString(value.system_identifier) &&
    hasBoolean(value, 'force_quirks') &&
    isHtmlSourceRange(value.source_range)
  )
}

function isHtmlDocumentSummary(value: unknown): value is HtmlDocumentSummary {
  if (!isObject(value)) {
    return false
  }

  const integerFields = [
    value.parsed_elements,
    value.source_start_tags,
    value.attributes,
    value.text_segments,
    value.comments,
    value.script_blocks,
    value.style_blocks,
    value.noscript_blocks,
    value.template_blocks,
    value.external_references,
    value.tokenizer_recoveries,
    value.tree_builder_recoveries,
    value.meta_elements,
    value.link_elements,
  ]

  return (
    (value.doctype === null || isHtmlDoctypeRecord(value.doctype)) &&
    hasString(value, 'quirks_mode') &&
    integerFields.every(isSafeInteger) &&
    hasBoolean(value, 'html_present') &&
    hasBoolean(value, 'head_present') &&
    hasBoolean(value, 'body_present') &&
    hasBoolean(value, 'title_present')
  )
}

function isHtmlMetaDeclaration(value: unknown): value is HtmlMetaDeclaration {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    isNullableString(value.charset) &&
    isNullableString(value.name) &&
    isNullableString(value.property) &&
    isNullableString(value.content) &&
    isHtmlSourceRange(value.source_range)
  )
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
}

function isHtmlMetadata(value: unknown): value is HtmlMetadata {
  return (
    isObject(value) &&
    isNullableString(value.title) &&
    isNullableString(value.lang) &&
    isNullableString(value.dir) &&
    Array.isArray(value.meta_declarations) &&
    value.meta_declarations.every(isHtmlMetaDeclaration) &&
    isStringArray(value.canonical_links) &&
    isStringArray(value.base_hrefs)
  )
}

function isHtmlAttributeRecord(value: unknown): value is HtmlAttributeRecord {
  return (
    isObject(value) &&
    hasString(value, 'name') &&
    hasString(value, 'value') &&
    isHtmlSourceRange(value.source_range)
  )
}

function isHtmlElementRecord(value: unknown): value is HtmlElementRecord {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    hasString(value, 'name') &&
    Array.isArray(value.attributes) &&
    value.attributes.every(isHtmlAttributeRecord) &&
    hasBoolean(value, 'self_closing') &&
    isHtmlSourceRange(value.source_range)
  )
}

function isHtmlTextClassification(value: unknown): value is HtmlTextClassification {
  return (
    value === 'visible_content_candidate' ||
    value === 'title' ||
    value === 'metadata' ||
    value === 'script_content' ||
    value === 'style_content' ||
    value === 'template_content' ||
    value === 'noscript_content'
  )
}

function isHtmlTextSegment(value: unknown): value is HtmlTextSegment {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    hasString(value, 'text') &&
    hasString(value, 'context_element') &&
    isHtmlTextClassification(value.classification) &&
    isNullableString(value.source_text) &&
    (value.source_range === null || isHtmlSourceRange(value.source_range))
  )
}

function isHtmlCommentRecord(value: unknown): value is HtmlCommentRecord {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    hasString(value, 'text') &&
    isHtmlSourceRange(value.source_range)
  )
}

function isHtmlReferenceKind(value: unknown): value is HtmlReferenceKind {
  return (
    value === 'relative' ||
    value === 'absolute' ||
    value === 'data_url' ||
    value === 'fragment' ||
    value === 'other'
  )
}

function isHtmlExternalReference(value: unknown): value is HtmlExternalReference {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    hasString(value, 'element') &&
    hasString(value, 'attribute') &&
    hasString(value, 'value') &&
    isHtmlReferenceKind(value.kind) &&
    isHtmlSourceRange(value.source_range)
  )
}

function isHtmlFindingKind(value: unknown): value is HtmlFindingKind {
  return (
    value === 'active_script_present' ||
    value === 'inline_event_handler_present' ||
    value === 'external_reference_present' ||
    value === 'explicit_hidden_marker' ||
    value === 'encoding_declaration_conflict' ||
    value === 'parser_recovery_observed'
  )
}

function isHtmlFinding(value: unknown): value is HtmlFinding {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    isHtmlFindingKind(value.kind) &&
    (value.severity === 'informational' || value.severity === 'caution') &&
    (value.element_sequence === null ||
      (isSafeInteger(value.element_sequence) && value.element_sequence > 0)) &&
    isNullableString(value.attribute) &&
    hasString(value, 'description') &&
    (value.source_range === null || isHtmlSourceRange(value.source_range))
  )
}

function isHtmlParserRecovery(value: unknown): value is HtmlParserRecovery {
  return (
    isObject(value) &&
    isSafeInteger(value.sequence) &&
    value.sequence > 0 &&
    (value.layer === 'tokenizer' || value.layer === 'tree_builder') &&
    hasString(value, 'message') &&
    (value.source_range === null || isHtmlSourceRange(value.source_range))
  )
}

function isHtmlTextChannels(value: unknown): value is HtmlTextChannels {
  return (
    isObject(value) &&
    hasString(value, 'document_text') &&
    hasString(value, 'metadata_text') &&
    hasString(value, 'active_content_text') &&
    hasString(value, 'style_text') &&
    hasString(value, 'template_text') &&
    hasString(value, 'noscript_text')
  )
}

function isHtmlAnalysis(value: unknown): value is HtmlAnalysis {
  if (
    !isObject(value) ||
    !isSafeInteger(value.raw_byte_length) ||
    !isTextDecodeResult(value.decode) ||
    !isHtmlEncodingEvidence(value.encoding) ||
    !isHtmlDocumentSummary(value.summary) ||
    !isHtmlMetadata(value.metadata) ||
    (value.provenance !== undefined && (!Array.isArray(value.provenance) || !value.provenance.every(isProvenanceEvidence))) ||
    !Array.isArray(value.elements) ||
    !value.elements.every(isHtmlElementRecord) ||
    !Array.isArray(value.text_segments) ||
    !value.text_segments.every(isHtmlTextSegment) ||
    !Array.isArray(value.comments) ||
    !value.comments.every(isHtmlCommentRecord) ||
    !Array.isArray(value.external_references) ||
    !value.external_references.every(isHtmlExternalReference) ||
    !Array.isArray(value.findings) ||
    !value.findings.every(isHtmlFinding) ||
    !Array.isArray(value.parser_recoveries) ||
    !value.parser_recoveries.every(isHtmlParserRecovery) ||
    !isHtmlTextChannels(value.text_channels) ||
    !isScanResult(value.scan)
  ) {
    return false
  }

  return (
    value.summary.source_start_tags === value.elements.length &&
    value.summary.text_segments === value.text_segments.length &&
    value.summary.comments === value.comments.length &&
    value.summary.external_references === value.external_references.length
  )
}

function isHtmlAnalysisErrorCode(value: unknown): value is HtmlAnalysisErrorCode {
  return (
    value === 'unsupported_encoding' ||
    value === 'ambiguous_encoding' ||
    value === 'invalid_utf8' ||
    value === 'invalid_utf16' ||
    value === 'unsupported_declared_encoding' ||
    value === 'resource_limit_exceeded' ||
    value === 'html_parser_failure'
  )
}

function isHtmlResourceLimit(value: unknown): value is HtmlResourceLimit {
  return (
    value === 'source_bytes' ||
    value === 'tokens' ||
    value === 'elements' ||
    value === 'attributes' ||
    value === 'nesting_depth' ||
    value === 'parse_recoveries' ||
    value === 'comments' ||
    value === 'attribute_value_bytes' ||
    value === 'comment_bytes' ||
    value === 'text_token_bytes' ||
    value === 'parsed_nodes'
  )
}

function isHtmlAnalysisError(value: unknown): value is HtmlAnalysisError {
  return (
    isObject(value) &&
    isHtmlAnalysisErrorCode(value.code) &&
    hasString(value, 'message') &&
    (value.resource_limit === null || isHtmlResourceLimit(value.resource_limit))
  )
}

function isHtmlAnalysisResponse(value: unknown): value is HtmlAnalysisResponse {
  if (!isObject(value)) {
    return false
  }

  return (
    (value.status === 'success' && isHtmlAnalysis(value.analysis)) ||
    (value.status === 'error' && isHtmlAnalysisError(value.error))
  )
}

export function parseScanResult(serialized: string): ScanResult {
  const parsed = parseJson(serialized)

  if (!isScanResult(parsed)) {
    throw new BridgeContractError('INVALID_SHAPE')
  }

  return parsed
}

export function parseCleanResult(serialized: string): CleanResult {
  const parsed = parseJson(serialized)

  if (!isCleanResult(parsed)) {
    throw new BridgeContractError('INVALID_SHAPE')
  }

  return parsed
}

export function parseCleanProfiles(serialized: string): CleanProfileDefinition[] {
  const parsed = parseJson(serialized)

  if (!Array.isArray(parsed) || !parsed.every(isCleanProfileDefinition)) {
    throw new BridgeContractError('INVALID_SHAPE')
  }

  return parsed
}

export function parseTextSourceAnalysisResponse(serialized: string): TextSourceAnalysisResponse {
  const parsed = parseJson(serialized)

  if (!isTextSourceAnalysisResponse(parsed)) {
    throw new BridgeContractError('INVALID_SHAPE')
  }

  return parsed
}

export function parseHtmlAnalysisResponse(serialized: string): HtmlAnalysisResponse {
  const parsed = parseJson(serialized)

  if (!isHtmlAnalysisResponse(parsed)) {
    throw new BridgeContractError('INVALID_SHAPE')
  }

  return parsed
}

function parseJson(serialized: string): unknown {
  try {
    return JSON.parse(serialized) as unknown
  } catch {
    throw new BridgeContractError('INVALID_JSON')
  }
}

export function summarizeScanResult(result: ScanResult): ScanSummary {
  return {
    characters: result.characters.length,
    invisible: result.characters.filter((record) => record.is_invisible).length,
    unusualWhitespace: result.characters.filter((record) => record.is_unusual_whitespace).length,
    bidiControls: result.characters.filter((record) => record.is_bidi_control).length,
  }
}
