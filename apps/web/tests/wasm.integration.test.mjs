import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'

import initializeWasm, {
  analyze_docx_bytes as analyzeDocxBytes,
  analyze_html_bytes as analyzeHtmlBytes,
  analyze_pdf_bytes as analyzePdfBytes,
  analyze_text_bytes as analyzeTextBytes,
  available_clean_profiles as availableCleanProfiles,
  clean_text as cleanText,
  reveal_text as revealText,
  scan_text as scanText,
  tcs_version as tcsVersion,
} from '../src/wasm/generated/textscope_wasm.js'
import { createDocx } from './docx-fixture.ts'
import { createPdf } from './pdf-fixture.ts'
import { parsePdfAnalysisResponse } from '../src/wasm/pdf-contract.ts'

const wasmBytes = readFileSync(
  new URL('../src/wasm/generated/textscope_wasm_bg.wasm', import.meta.url),
)

await initializeWasm({ module_or_path: wasmBytes })

function scan(text) {
  return JSON.parse(scanText(text))
}

function clean(text, profile) {
  return JSON.parse(cleanText(text, profile))
}

function analyzeSource(bytes, sourceKind = 'plain_text') {
  return JSON.parse(analyzeTextBytes(Uint8Array.from(bytes), sourceKind))
}

function analyzeHtml(source) {
  const bytes = typeof source === 'string' ? new TextEncoder().encode(source) : source
  return JSON.parse(analyzeHtmlBytes(Uint8Array.from(bytes)))
}

function analyzeDocx(bytes) {
  return JSON.parse(analyzeDocxBytes(Uint8Array.from(bytes)))
}

function analyzePdf(bytes) {
  return JSON.parse(analyzePdfBytes(Uint8Array.from(bytes)))
}

test('PDF bytes cross the real WASM boundary into pages, objects, metadata, and text', () => {
  const response = analyzePdf(createPdf({ pages: ['Hello PDF', 'Second page'], metadata: true, xmp: true }))

  assert.equal(response.status, 'success')
  assert.equal(parsePdfAnalysisResponse(JSON.stringify(response)).status, 'success')
  assert.equal(response.analysis.summary.page_count, 2)
  assert.equal(response.analysis.summary.object_count > 0, true)
  assert.equal(response.analysis.summary.metadata_present, true)
  assert.equal(response.analysis.metadata.title, 'Declared title')
  assert.equal(response.analysis.metadata.xmp?.well_formed_xml, true)
  assert.equal(response.analysis.document_text, 'Hello PDF\n\nSecond page')
  assert.equal(response.analysis.text_extraction_status, 'extracted')
  assert.equal(response.analysis.pages[0].text, 'Hello PDF')
})

test('PDF actions, URI references, attachments, forms, and signatures remain structural and inert', () => {
  const response = analyzePdf(createPdf({ javascript: true, uri: true, launch: true, submitForm: true, attachment: true, form: true, signature: true }))

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.summary.javascript_present, true)
  assert.equal(response.analysis.actions.some(({ kind }) => kind === 'javascript'), true)
  assert.equal(response.analysis.actions.some(({ kind }) => kind === 'uri'), true)
  assert.equal(response.analysis.actions.some(({ kind }) => kind === 'launch'), true)
  assert.equal(response.analysis.actions.some(({ kind }) => kind === 'submit_form'), true)
  assert.equal(response.analysis.external_references[0].value, 'https://example.invalid/textscope')
  assert.equal(response.analysis.embedded_files.length, 1)
  assert.equal(response.analysis.forms.present, true)
  assert.equal(response.analysis.forms.xfa_present, true)
  assert.equal(response.analysis.signatures.length >= 1, true)
})

test('PDF malformed and empty bytes return controlled boundary errors without echoing bytes', () => {
  for (const bytes of [[], Buffer.from('not a PDF', 'utf8')]) {
    const response = analyzePdf(bytes)
    assert.equal(response.status, 'error')
    assert.equal(['empty_input', 'non_pdf_bytes', 'invalid_header', 'malformed_structure'].includes(response.error.code), true)
    assert.equal(JSON.stringify(response).includes('not a PDF'), false)
  }
})

test('PDF text extraction reports no text layer without fabricating OCR', () => {
  const response = analyzePdf(createPdf({ pages: [''] }))
  assert.equal(response.status, 'success')
  assert.equal(response.analysis.text_extraction_status, 'no_text_operators')
  assert.equal(response.analysis.findings.some(({ kind }) => kind === 'no_text_layer'), true)
})

test('DOCX bytes cross the real WASM boundary into package, body, and scan data', () => {
  const response = analyzeDocx(createDocx({
    bodyXml: '<w:p><w:r><w:t>A\u00a0B\u200b😀</w:t></w:r></w:p>',
  }))

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.package.status, 'valid_wordprocessing_package')
  assert.equal(response.analysis.text_channels.document_text, 'A\u00a0B\u200b😀')
  assert.deepEqual(response.analysis.scan.findings.map(({ kind }) => kind), [
    'unusual_whitespace',
    'invisible_character',
  ])
})

test('DOCX metadata, external relationships, and comments stay structured and inert', () => {
  const response = analyzeDocx(createDocx({
    documentRelationships: '<Relationship Id="rLink" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/never" TargetMode="External"/>',
    extraEntries: [
      ['docProps/core.xml', '<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:creator>Declared creator</dc:creator></cp:coreProperties>'],
      ['word/comments.xml', '<w:comments xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:comment w:id="1" w:author="Declared reviewer"><w:p><w:r><w:t>Separate comment</w:t></w:r></w:p></w:comment></w:comments>'],
    ],
  }))

  assert.equal(response.analysis.metadata.core_properties[0].value, 'Declared creator')
  assert.equal(response.analysis.relationships[1].external, true)
  assert.equal(response.analysis.comments[0].text, 'Separate comment')
  assert.equal(response.analysis.text_channels.document_text.includes('Separate comment'), false)
})

test('invalid and empty DOCX byte arrays return controlled boundary errors', () => {
  for (const bytes of [[], [1, 2, 3]]) {
    const response = analyzeDocx(bytes)
    assert.equal(response.status, 'error')
    assert.equal(response.error.code, 'invalid_zip')
  }
})

test('HTML UTF-8 bytes cross the real WASM boundary into structured metadata', () => {
  const response = analyzeHtml(
    '<!doctype html><html lang="pt-BR"><head><meta charset="utf-8"><title>Relatório</title></head><body>Texto</body></html>',
  )

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.metadata.title, 'Relatório')
  assert.equal(response.analysis.metadata.lang, 'pt-BR')
  assert.equal(response.analysis.summary.doctype.name, 'html')
  assert.equal(response.analysis.text_channels.document_text, 'Texto')
})

test('HTML entities retain source representation and feed decoded Unicode to scan', () => {
  const response = analyzeHtml('<p>A&amp;B&nbsp;C&#x200B;D</p>')

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.text_channels.document_text, 'A&B\u00a0C\u200bD')
  assert.equal(response.analysis.text_segments[0].source_text, 'A&amp;B&nbsp;C&#x200B;D')
  assert.deepEqual(
    response.analysis.scan.findings.map(({ kind }) => kind),
    ['unusual_whitespace', 'invisible_character'],
  )
})

test('HTML scripts and comments stay in separate inert channels', () => {
  const response = analyzeHtml(
    '<body>A<!-- note --><script>window.__ran=true</script>B</body>',
  )

  assert.equal(response.analysis.text_channels.document_text, 'AB')
  assert.equal(response.analysis.text_channels.active_content_text, 'window.__ran=true')
  assert.equal(response.analysis.comments[0].text, ' note ')
  assert.equal(response.analysis.findings[0].kind, 'active_script_present')
})

test('HTML declared and invalid encoding failures are controlled at WASM boundary', () => {
  const unsupported = analyzeHtml('<meta charset=windows-1252><p>A</p>')
  const invalid = analyzeHtml([0xc3, 0x28])

  assert.equal(unsupported.status, 'error')
  assert.equal(unsupported.error.code, 'unsupported_declared_encoding')
  assert.equal(invalid.status, 'error')
  assert.equal(invalid.error.code, 'invalid_utf8')
})

test('empty HTML bytes are a valid Uint8Array analysis', () => {
  const response = analyzeHtml([])

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.raw_byte_length, 0)
  assert.equal(response.analysis.text_channels.document_text, '')
  assert.deepEqual(response.analysis.scan.characters, [])
})

test('raw UTF-8 bytes cross WASM without Base64 and retain source ranges', () => {
  const bytes = new TextEncoder().encode('A😀B')
  const response = analyzeSource(bytes)

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.raw_byte_length, 6)
  assert.equal(response.analysis.decode.encoding, 'utf_8')
  assert.equal(response.analysis.decode.bom, 'none')
  assert.equal(response.analysis.decode.decoded_text, 'A😀B')
  assert.deepEqual(response.analysis.decode.source_mappings, [
    {
      code_point_index: 0,
      source_byte_range: { start: 0, end: 1 },
      decoded_utf8_byte_offset: 0,
      utf16_offset: 0,
    },
    {
      code_point_index: 1,
      source_byte_range: { start: 1, end: 5 },
      decoded_utf8_byte_offset: 1,
      utf16_offset: 1,
    },
    {
      code_point_index: 2,
      source_byte_range: { start: 5, end: 6 },
      decoded_utf8_byte_offset: 5,
      utf16_offset: 3,
    },
  ])
})

test('UTF-8 BOM is metadata rather than a decoded U+FEFF character', () => {
  const response = analyzeSource([0xef, 0xbb, 0xbf, 0x4f, 0x6c, 0xc3, 0xa1])

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.decode.bom, 'utf_8')
  assert.equal(response.analysis.decode.encoding_evidence, 'byte_order_mark')
  assert.equal(response.analysis.decode.decoded_text, 'Olá')
  assert.deepEqual(response.analysis.decode.source_mappings[0].source_byte_range, {
    start: 3,
    end: 4,
  })
})

test('UTF-16 LE bytes decode strictly and preserve a surrogate-pair source range', () => {
  const response = analyzeSource([0xff, 0xfe, 0x41, 0x00, 0x3d, 0xd8, 0x00, 0xde, 0x42, 0x00])

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.decode.encoding, 'utf_16_le')
  assert.equal(response.analysis.decode.decoded_text, 'A😀B')
  assert.deepEqual(response.analysis.decode.source_mappings[1], {
    code_point_index: 1,
    source_byte_range: { start: 4, end: 8 },
    decoded_utf8_byte_offset: 1,
    utf16_offset: 1,
  })
})

test('UTF-16 BE bytes decode strictly through the real WASM boundary', () => {
  const response = analyzeSource([0xfe, 0xff, 0x00, 0x4f, 0x00, 0x6c, 0x00, 0xe1], 'markdown')

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.source_kind, 'markdown')
  assert.equal(response.analysis.decode.encoding, 'utf_16_be')
  assert.equal(response.analysis.decode.decoded_text, 'Olá')
})

test('byte ingestion returns controlled errors without echoing source bytes', () => {
  const cases = [
    { bytes: [0xff], code: 'invalid_utf8' },
    { bytes: [0x41, 0x00, 0x42, 0x00], code: 'ambiguous_encoding' },
    { bytes: [0xff, 0xfe, 0x00, 0x00], code: 'unsupported_encoding' },
  ]

  for (const { bytes, code } of cases) {
    const response = analyzeSource(bytes)
    assert.equal(response.status, 'error')
    assert.equal(response.error.code, code)
    assert.equal(JSON.stringify(response).includes(JSON.stringify(bytes)), false)
  }
})

test('empty byte input is a valid, explicit UTF-8 decision', () => {
  const response = analyzeSource([])

  assert.equal(response.status, 'success')
  assert.equal(response.analysis.decode.encoding_evidence, 'empty_input')
  assert.equal(response.analysis.decode.decoded_text, '')
  assert.deepEqual(response.analysis.scan.characters, [])
})

test('WASM module initializes and preserves A😀B offsets', () => {
  const result = scan('A😀B')

  assert.deepEqual(
    result.characters.map(({ code_point_index, utf16_offset, utf8_byte_offset }) => ({
      code_point_index,
      utf16_offset,
      utf8_byte_offset,
    })),
    [
      { code_point_index: 0, utf16_offset: 0, utf8_byte_offset: 0 },
      { code_point_index: 1, utf16_offset: 1, utf8_byte_offset: 1 },
      { code_point_index: 2, utf16_offset: 3, utf8_byte_offset: 5 },
    ],
  )
})

test('U+200B crosses the boundary as invisible, finding, and core reveal', () => {
  const result = scan('\u200B')

  assert.equal(result.characters.filter((record) => record.is_invisible).length, 1)
  assert.equal(result.findings.length, 1)
  assert.equal(revealText('\u200B'), '⟦ZWSP⟧')
})

test('U+00A0 unusual whitespace count comes from the core flag', () => {
  const result = scan('\u00A0')

  assert.equal(result.characters.filter((record) => record.is_unusual_whitespace).length, 1)
})

test('U+202E bidi count and finding cross the boundary', () => {
  const result = scan('\u202E')

  assert.equal(result.characters.filter((record) => record.is_bidi_control).length, 1)
  assert.equal(result.findings[0]?.kind, 'bidirectional_control')
})

test('emoji ZWJ sequence keeps the core conservative observation', () => {
  const result = scan('👩\u200D💻')
  const joiner = result.characters.find((record) => record.code_point === 0x200d)

  assert.equal(joiner?.is_invisible, true)
  assert.equal(result.findings[0]?.kind, 'invisible_character')
})

test('normal Portuguese text gains no frontend findings', () => {
  assert.deepEqual(scan('Olá, ação e coração.').findings, [])
})

test('empty input remains valid', () => {
  const result = scan('')

  assert.deepEqual(result.characters, [])
  assert.deepEqual(result.findings, [])
  assert.equal(revealText(''), '')
})

test('TCS version and profile registry cross the real WASM boundary', () => {
  const profiles = JSON.parse(availableCleanProfiles())

  assert.equal(tcsVersion(), '0.1.0')
  assert.deepEqual(
    profiles.filter(({ supported }) => supported).map(({ id }) => id),
    ['safe_clean', 'canonical_clean', 'pure_plain_text'],
  )
  assert.deepEqual(
    profiles.filter(({ supported }) => !supported).map(({ id }) => id),
    ['document_clean', 'developer_clean'],
  )
})

test('normal text stays unchanged through Safe Clean and verification', () => {
  const input = 'Olá, ação e coração.'
  const result = clean(input, 'safe_clean')

  assert.equal(result.original_text, input)
  assert.equal(result.clean_text, input)
  assert.deepEqual(result.manifest.transformations, [])
  assert.equal(result.verification.verification_status, 'verified')
})

test('Canonical Clean returns real Rust normalization manifest and rescan', () => {
  const result = clean('cafe\u0301', 'canonical_clean')

  assert.equal(result.clean_text, 'café')
  assert.equal(result.manifest.transformations.length, 1)
  assert.equal(result.manifest.transformations[0].rule_id, 'TCS-NORMALIZATION-001')
  assert.deepEqual(result.manifest.transformations[0].original_code_point_range, {
    start: 3,
    end: 5,
  })
  assert.equal(result.verification.clean_scan_summary.characters, 4)
  assert.equal(result.verification.checks.clean_rescan_completed, true)
  assert.equal(result.verification.checks.idempotent, true)
})

test('Pure Plain Text applies the explicit NBSP policy with review status', () => {
  const result = clean('Olá\u00A0mundo', 'pure_plain_text')

  assert.equal(result.clean_text, 'Olá mundo')
  assert.equal(result.manifest.transformations[0].rule_id, 'TCS-WHITESPACE-001')
  assert.equal(result.content_preservation_diff.status, 'needs_review')
  assert.equal(result.verification.verification_status, 'needs_review')
})

test('clean profiles preserve ambiguous Unicode observations', () => {
  const input = '👩\u200D💻\u200C✈\uFE0F\u202E\u00AD\u2060'

  for (const profile of ['safe_clean', 'canonical_clean', 'pure_plain_text']) {
    const result = clean(input, profile)
    assert.equal(result.clean_text, input)
    assert.deepEqual(result.manifest.transformations, [])
    assert.equal(result.verification.remaining_findings.length, 6)
    assert.equal(result.verification.checks.preserved_findings_visible, true)
  }
})

test('cleaning through WASM is idempotent and rejects unknown profiles safely', () => {
  const first = clean('\uFEFFe\u0301\r\n', 'canonical_clean')
  const second = clean(first.clean_text, 'canonical_clean')
  const sensitive = 'private pasted text'

  assert.equal(second.clean_text, first.clean_text)
  assert.deepEqual(second.manifest.transformations, [])
  assert.throws(() => cleanText(sensitive, 'unknown_profile'), (error) => {
    const message = String(error)
    return (
      message === 'TextScope received an unknown clean profile.' && !message.includes(sensitive)
    )
  })
})
