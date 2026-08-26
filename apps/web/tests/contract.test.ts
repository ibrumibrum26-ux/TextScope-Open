import assert from 'node:assert/strict'
import test from 'node:test'

import {
  BridgeContractError,
  parseCleanProfiles,
  parseCleanResult,
  parseHtmlAnalysisResponse,
  parseScanResult,
  parseTextSourceAnalysisResponse,
  summarizeScanResult,
  type CleanResult,
  type HtmlAnalysisResponse,
  type ScanResult,
  type TextSourceAnalysisResponse,
} from '../src/wasm/contract.ts'
import { parseDocxAnalysisResponse, type DocxAnalysisResponse } from '../src/wasm/docx-contract.ts'
import { parsePdfAnalysisResponse } from '../src/wasm/pdf-contract.ts'

const emptyResult: ScanResult = {
  characters: [],
  findings: [],
  normalization: {
    nfc_would_change: false,
    nfd_would_change: false,
    nfkc_would_change: false,
    nfkd_would_change: false,
  },
}

const emptyDocxResponse: DocxAnalysisResponse = {
  status: 'success',
  analysis: {
    package: {
      status: 'valid_wordprocessing_package',
      raw_byte_length: 0,
      entry_count: 0,
      compressed_total: 0,
      uncompressed_total: 0,
      compression_ratio_milli: 0,
      zip64_present: false,
      archive_offset: 0,
      duplicate_entry_names: 0,
      invalid_entry_names: 0,
    },
    parts: [],
    content_types: [],
    relationships: [],
    metadata: { core_properties: [], app_properties: [], custom_properties: [] },
    summary: {
      main_document_present: true,
      styles_present: false,
      numbering_present: false,
      paragraphs: 0,
      runs: 0,
      tables: 0,
      table_rows: 0,
      table_cells: 0,
      headers: 0,
      footers: 0,
      footnotes: 0,
      endnotes: 0,
      comments: 0,
      tracked_changes: 0,
      explicit_hidden_runs: 0,
      hyperlinks: 0,
      external_relationships: 0,
      media_parts: 0,
      embedded_object_parts: 0,
      custom_xml_parts: 0,
    },
    text_segments: [],
    comments: [],
    revisions: [],
    hyperlinks: [],
    findings: [],
    text_channels: {
      document_text: '', headers: [], footers: [], footnotes: [], endnotes: [], comments: [],
      deleted_content: '',
    },
    scan: emptyResult,
  },
}

const emptyCleanResult: CleanResult = {
  original_text: '',
  clean_text: '',
  profile: 'safe_clean',
  tcs_version: '0.1.0',
  manifest: {
    tcs_version: '0.1.0',
    profile: 'safe_clean',
    transformations: [],
  },
  technical_diff: {
    changed: false,
    original_utf8_bytes: 0,
    clean_utf8_bytes: 0,
    original_code_points: 0,
    clean_code_points: 0,
    transformation_count: 0,
  },
  content_preservation_diff: {
    status: 'preserved',
    reason: 'No text transformation was applied.',
  },
  verification: {
    original_scan_summary: {
      characters: 0,
      findings: 0,
      invisible: 0,
      unusual_whitespace: 0,
      bidi_controls: 0,
      variation_selectors: 0,
    },
    clean_scan_summary: {
      characters: 0,
      findings: 0,
      invisible: 0,
      unusual_whitespace: 0,
      bidi_controls: 0,
      variation_selectors: 0,
    },
    applied_transformations: 0,
    remaining_findings: [],
    content_preservation_status: 'preserved',
    verification_status: 'verified',
    checks: {
      manifest_reproduces_clean_text: true,
      rules_allowed_by_profile: true,
      no_undeclared_transformations: true,
      clean_rescan_completed: true,
      expected_findings_reassessed: true,
      preserved_findings_visible: true,
      deterministic: true,
      idempotent: true,
    },
  },
}

test('parses the explicit bridge contract and accepts empty input', () => {
  assert.deepEqual(parseScanResult(JSON.stringify(emptyResult)), emptyResult)
})

test('rejects malformed JSON without retaining the input in its message', () => {
  const malformed = 'sensitive input that is not JSON'

  assert.throws(
    () => parseScanResult(malformed),
    (error: unknown) =>
      error instanceof BridgeContractError &&
      error.code === 'INVALID_JSON' &&
      !error.message.includes(malformed),
  )
})

test('rejects a malformed contract with a controlled error', () => {
  assert.throws(
    () => parseScanResult('{"characters":[]}'),
    (error: unknown) => error instanceof BridgeContractError && error.code === 'INVALID_SHAPE',
  )
})

test('summary counts only flags supplied by the core', () => {
  const result: ScanResult = {
    ...emptyResult,
    characters: [
      {
        code_point_index: 0,
        utf16_offset: 0,
        utf8_byte_offset: 0,
        character: 'x',
        code_point: 120,
        code_point_label: 'U+0078 LATIN SMALL LETTER X',
        utf8_bytes: [120],
        unicode_category: 'Ll',
        script: 'Latin',
        is_whitespace: false,
        is_control: false,
        is_format: false,
        is_invisible: true,
        is_unusual_whitespace: true,
        is_bidi_control: true,
        is_variation_selector: false,
      },
    ],
  }

  assert.deepEqual(summarizeScanResult(result), {
    characters: 1,
    invisible: 1,
    unusualWhitespace: 1,
    bidiControls: 1,
  })
})

test('parses the complete clean, manifest, diff, and verification contract', () => {
  assert.deepEqual(parseCleanResult(JSON.stringify(emptyCleanResult)), emptyCleanResult)
})

test('rejects an incomplete clean contract with a controlled error', () => {
  assert.throws(
    () => parseCleanResult('{"original_text":"sensitive","clean_text":""}'),
    (error: unknown) =>
      error instanceof BridgeContractError &&
      error.code === 'INVALID_SHAPE' &&
      !error.message.includes('sensitive'),
  )
})

test('rejects an incomplete PDF bridge response without echoing source data', () => {
  const sensitive = '%PDF-1.7 private bytes'
  assert.throws(
    () => parsePdfAnalysisResponse(JSON.stringify({ status: 'success', analysis: { sensitive } })),
    (error: unknown) =>
      error instanceof BridgeContractError &&
      error.code === 'INVALID_SHAPE' &&
      !error.message.includes(sensitive),
  )
})

test('parses supported and reserved clean profile metadata from Rust', () => {
  const profiles = [
    {
      id: 'safe_clean',
      label: 'Safe Clean',
      description: 'Conservative',
      supported: true,
      active_rule_ids: ['TCS-LINE-ENDINGS-001'],
    },
    {
      id: 'document_clean',
      label: 'Document Clean',
      description: 'Reserved',
      supported: false,
      active_rule_ids: [],
    },
  ]

  assert.deepEqual(parseCleanProfiles(JSON.stringify(profiles)), profiles)
})

test('rejects unknown profile identifiers instead of inventing frontend behavior', () => {
  assert.throws(
    () =>
      parseCleanProfiles(
        '[{"id":"aggressive_clean","label":"Unknown","description":"x","supported":true,"active_rule_ids":[]}]',
      ),
    (error: unknown) => error instanceof BridgeContractError && error.code === 'INVALID_SHAPE',
  )
})

const emptySourceResponse: TextSourceAnalysisResponse = {
  status: 'success',
  analysis: {
    raw_byte_length: 0,
    source_kind: 'plain_text',
    decode: {
      encoding: 'utf_8',
      encoding_evidence: 'empty_input',
      bom: 'none',
      status: 'decoded',
      decoded_text: '',
      source_mappings: [],
    },
    scan: emptyResult,
  },
}

test('parses strict source analysis with separate source and decoded offsets', () => {
  const response: TextSourceAnalysisResponse = {
    status: 'success',
    analysis: {
      raw_byte_length: 10,
      source_kind: 'plain_text',
      decode: {
        encoding: 'utf_16_le',
        encoding_evidence: 'byte_order_mark',
        bom: 'utf_16_le',
        status: 'decoded',
        decoded_text: 'A😀B',
        source_mappings: [
          {
            code_point_index: 0,
            source_byte_range: { start: 2, end: 4 },
            decoded_utf8_byte_offset: 0,
            utf16_offset: 0,
          },
          {
            code_point_index: 1,
            source_byte_range: { start: 4, end: 8 },
            decoded_utf8_byte_offset: 1,
            utf16_offset: 1,
          },
          {
            code_point_index: 2,
            source_byte_range: { start: 8, end: 10 },
            decoded_utf8_byte_offset: 5,
            utf16_offset: 3,
          },
        ],
      },
      scan: {
        ...emptyResult,
        characters: [
          {
            code_point_index: 0,
            utf16_offset: 0,
            utf8_byte_offset: 0,
            character: 'A',
            code_point: 65,
            code_point_label: 'U+0041 LATIN CAPITAL LETTER A',
            utf8_bytes: [65],
            unicode_category: 'Lu',
            script: 'Latin',
            is_whitespace: false,
            is_control: false,
            is_format: false,
            is_invisible: false,
            is_unusual_whitespace: false,
            is_bidi_control: false,
            is_variation_selector: false,
          },
          {
            code_point_index: 1,
            utf16_offset: 1,
            utf8_byte_offset: 1,
            character: '😀',
            code_point: 0x1f600,
            code_point_label: 'U+1F600 GRINNING FACE',
            utf8_bytes: [240, 159, 152, 128],
            unicode_category: 'So',
            script: 'Common',
            is_whitespace: false,
            is_control: false,
            is_format: false,
            is_invisible: false,
            is_unusual_whitespace: false,
            is_bidi_control: false,
            is_variation_selector: false,
          },
          {
            code_point_index: 2,
            utf16_offset: 3,
            utf8_byte_offset: 5,
            character: 'B',
            code_point: 66,
            code_point_label: 'U+0042 LATIN CAPITAL LETTER B',
            utf8_bytes: [66],
            unicode_category: 'Lu',
            script: 'Latin',
            is_whitespace: false,
            is_control: false,
            is_format: false,
            is_invisible: false,
            is_unusual_whitespace: false,
            is_bidi_control: false,
            is_variation_selector: false,
          },
        ],
      },
    },
  }

  assert.deepEqual(parseTextSourceAnalysisResponse(JSON.stringify(response)), response)
})

test('parses empty source and controlled ingestion errors', () => {
  assert.deepEqual(
    parseTextSourceAnalysisResponse(JSON.stringify(emptySourceResponse)),
    emptySourceResponse,
  )

  const error: TextSourceAnalysisResponse = {
    status: 'error',
    error: { code: 'ambiguous_encoding', message: 'Explicit encoding is required.' },
  }
  assert.deepEqual(parseTextSourceAnalysisResponse(JSON.stringify(error)), error)
})

test('rejects malformed source analysis without echoing source content', () => {
  const malformed = '{"status":"success","analysis":{"decoded_text":"private"}}'
  assert.throws(
    () => parseTextSourceAnalysisResponse(malformed),
    (error: unknown) =>
      error instanceof BridgeContractError &&
      error.code === 'INVALID_SHAPE' &&
      !error.message.includes('private'),
  )
})

const emptyHtmlResponse: HtmlAnalysisResponse = {
  status: 'success',
  analysis: {
    raw_byte_length: 0,
    decode: {
      encoding: 'utf_8',
      encoding_evidence: 'empty_input',
      bom: 'none',
      status: 'decoded',
      decoded_text: '',
      source_mappings: [],
    },
    encoding: {
      declarations: [],
      bom_precedence_applied: false,
      declaration_conflict: false,
    },
    summary: {
      doctype: null,
      quirks_mode: 'Quirks',
      parsed_elements: 3,
      source_start_tags: 0,
      attributes: 0,
      text_segments: 0,
      comments: 0,
      script_blocks: 0,
      style_blocks: 0,
      noscript_blocks: 0,
      template_blocks: 0,
      external_references: 0,
      tokenizer_recoveries: 0,
      tree_builder_recoveries: 1,
      html_present: true,
      head_present: true,
      body_present: true,
      title_present: false,
      meta_elements: 0,
      link_elements: 0,
    },
    metadata: {
      title: null,
      lang: null,
      dir: null,
      meta_declarations: [],
      canonical_links: [],
      base_hrefs: [],
    },
    elements: [],
    text_segments: [],
    comments: [],
    external_references: [],
    findings: [],
    parser_recoveries: [],
    text_channels: {
      document_text: '',
      metadata_text: '',
      active_content_text: '',
      style_text: '',
      template_text: '',
      noscript_text: '',
    },
    scan: emptyResult,
  },
}

test('parses the structured HTML analysis contract and empty document text', () => {
  assert.deepEqual(parseHtmlAnalysisResponse(JSON.stringify(emptyHtmlResponse)), emptyHtmlResponse)
})

test('parses controlled HTML errors and rejects incomplete HTML structures', () => {
  const error: HtmlAnalysisResponse = {
    status: 'error',
    error: {
      code: 'resource_limit_exceeded',
      message: 'Explicit limit reached.',
      resource_limit: 'nesting_depth',
    },
  }
  assert.deepEqual(parseHtmlAnalysisResponse(JSON.stringify(error)), error)

  const malformed = '{"status":"success","analysis":{"decoded_source":"private"}}'
  assert.throws(
    () => parseHtmlAnalysisResponse(malformed),
    (contractError: unknown) =>
      contractError instanceof BridgeContractError &&
      contractError.code === 'INVALID_SHAPE' &&
      !contractError.message.includes('private'),
  )
})

test('parses the typed DOCX response and controlled resource errors', () => {
  assert.deepEqual(parseDocxAnalysisResponse(JSON.stringify(emptyDocxResponse)), emptyDocxResponse)

  const error: DocxAnalysisResponse = {
    status: 'error',
    error: {
      code: 'resource_limit_exceeded',
      message: 'Explicit package limit reached.',
      resource_limit: 'compression_ratio',
    },
  }
  assert.deepEqual(parseDocxAnalysisResponse(JSON.stringify(error)), error)
})

test('rejects incomplete DOCX boundary data without echoing document content', () => {
  const malformed = '{"status":"success","analysis":{"text":"private DOCX content"}}'
  assert.throws(
    () => parseDocxAnalysisResponse(malformed),
    (error: unknown) =>
      error instanceof BridgeContractError &&
      error.code === 'INVALID_SHAPE' &&
      !error.message.includes('private DOCX content'),
  )
})
