import assert from 'node:assert/strict'
import test from 'node:test'

import {
  LocalFileInputError,
  MAX_LOCAL_DOCX_FILE_BYTES,
  MAX_LOCAL_HTML_FILE_BYTES,
  MAX_LOCAL_PDF_FILE_BYTES,
  MAX_LOCAL_TEXT_FILE_BYTES,
  inspectLocalHtmlFile,
  inspectLocalDocxFile,
  inspectLocalPdfFile,
  inspectLocalTextFile,
  loadLocalHtmlFile,
  loadLocalDocxFile,
  loadLocalPdfFile,
  loadLocalTextFile,
} from '../src/local-file.ts'

test('selects TXT through the extension allowlist and preserves advisory metadata', () => {
  assert.deepEqual(
    inspectLocalTextFile({ name: 'Relatório.TXT', size: 42, type: 'application/octet-stream' }),
    {
      file_name: 'Relatório.TXT',
      extension: '.txt',
      browser_reported_mime_type: 'application/octet-stream',
      byte_size: 42,
      source_kind: 'plain_text',
      source_kind_label: 'TXT',
    },
  )
})

test('selects only DOCX and rejects macro-enabled DOCM explicitly', () => {
  assert.deepEqual(
    inspectLocalDocxFile({ name: 'Relatório.DOCX', size: 128, type: 'application/octet-stream' }),
    {
      file_name: 'Relatório.DOCX',
      extension: '.docx',
      browser_reported_mime_type: 'application/octet-stream',
      byte_size: 128,
      source_kind_label: 'DOCX',
    },
  )
  assert.throws(
    () => inspectLocalDocxFile({ name: 'macro.docm', size: 128, type: '' }),
    (error: unknown) =>
      error instanceof LocalFileInputError && error.code === 'unsupported_file_type',
  )
})

test('selects PDF by extension and keeps browser MIME advisory', () => {
  assert.deepEqual(
    inspectLocalPdfFile({ name: 'evidence.PDF', size: 256, type: 'application/octet-stream' }),
    {
      file_name: 'evidence.PDF',
      extension: '.pdf',
      browser_reported_mime_type: 'application/octet-stream',
      byte_size: 256,
      source_kind_label: 'PDF',
    },
  )
  assert.throws(
    () => inspectLocalPdfFile({ name: 'evidence.txt', size: 1, type: 'text/plain' }),
    (error: unknown) => error instanceof LocalFileInputError && error.code === 'unsupported_file_type',
  )
})

test('selects both Markdown extensions as literal Markdown sources', () => {
  for (const name of ['notes.md', 'notes.markdown']) {
    const metadata = inspectLocalTextFile({ name, size: 10, type: 'text/markdown' })
    assert.equal(metadata.source_kind, 'markdown')
    assert.equal(metadata.source_kind_label, 'Markdown')
  }
})

test('selects both HTML extensions only in the dedicated inert adapter', () => {
  for (const name of ['page.HTML', 'archive.htm']) {
    const metadata = inspectLocalHtmlFile({ name, size: 12, type: 'text/html' })
    assert.equal(metadata.source_kind_label, 'HTML')
    assert.equal(metadata.extension, name.toLowerCase().endsWith('.html') ? '.html' : '.htm')
  }

  assert.throws(
    () => inspectLocalHtmlFile({ name: 'page.txt', size: 1, type: 'text/plain' }),
    (error: unknown) =>
      error instanceof LocalFileInputError && error.code === 'unsupported_file_type',
  )
})

test('rejects extensions outside the explicit adapter scope', () => {
  for (const name of ['page.html', 'document.pdf', 'README']) {
    assert.throws(
      () => inspectLocalTextFile({ name, size: 1, type: 'text/plain' }),
      (error: unknown) =>
        error instanceof LocalFileInputError && error.code === 'unsupported_file_type',
    )
  }
})

test('accepts exactly 10 MiB and rejects one byte above the UI limit', () => {
  assert.equal(
    inspectLocalTextFile({ name: 'limit.txt', size: MAX_LOCAL_TEXT_FILE_BYTES, type: '' })
      .byte_size,
    MAX_LOCAL_TEXT_FILE_BYTES,
  )
  assert.throws(
    () =>
      inspectLocalTextFile({
        name: 'too-large.txt',
        size: MAX_LOCAL_TEXT_FILE_BYTES + 1,
        type: '',
      }),
    (error: unknown) => error instanceof LocalFileInputError && error.code === 'file_too_large',
  )
})

test('checks file size before calling arrayBuffer', async () => {
  let readAttempted = false
  const oversizedFile = {
    name: 'too-large.txt',
    size: MAX_LOCAL_TEXT_FILE_BYTES + 1,
    type: 'text/plain',
    async arrayBuffer() {
      readAttempted = true
      return new ArrayBuffer(0)
    },
  } as unknown as File

  await assert.rejects(
    loadLocalTextFile(oversizedFile),
    (error: unknown) => error instanceof LocalFileInputError && error.code === 'file_too_large',
  )
  assert.equal(readAttempted, false)
})

test('checks HTML size before calling arrayBuffer', async () => {
  let readAttempted = false
  const oversizedFile = {
    name: 'too-large.html',
    size: MAX_LOCAL_HTML_FILE_BYTES + 1,
    type: 'text/html',
    async arrayBuffer() {
      readAttempted = true
      return new ArrayBuffer(0)
    },
  } as unknown as File

  await assert.rejects(
    loadLocalHtmlFile(oversizedFile),
    (error: unknown) => error instanceof LocalFileInputError && error.code === 'file_too_large',
  )
  assert.equal(readAttempted, false)
})

test('checks DOCX size before calling arrayBuffer', async () => {
  let readAttempted = false
  const oversizedFile = {
    name: 'too-large.docx',
    size: MAX_LOCAL_DOCX_FILE_BYTES + 1,
    type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    async arrayBuffer() {
      readAttempted = true
      return new ArrayBuffer(0)
    },
  } as unknown as File

  await assert.rejects(
    loadLocalDocxFile(oversizedFile),
    (error: unknown) => error instanceof LocalFileInputError && error.code === 'file_too_large',
  )
  assert.equal(readAttempted, false)
})

test('checks PDF size before calling arrayBuffer', async () => {
  let readAttempted = false
  const oversizedFile = {
    name: 'too-large.pdf',
    size: MAX_LOCAL_PDF_FILE_BYTES + 1,
    type: 'application/pdf',
    async arrayBuffer() {
      readAttempted = true
      return new ArrayBuffer(0)
    },
  } as unknown as File

  await assert.rejects(
    loadLocalPdfFile(oversizedFile),
    (error: unknown) => error instanceof LocalFileInputError && error.code === 'file_too_large',
  )
  assert.equal(readAttempted, false)
})

test('maps browser read failures to a controlled content-free error', async () => {
  const unreadableFile = {
    name: 'notes.txt',
    size: 1,
    type: 'text/plain',
    async arrayBuffer(): Promise<ArrayBuffer> {
      throw new Error('private browser detail')
    },
  } as unknown as File

  await assert.rejects(
    loadLocalTextFile(unreadableFile),
    (error: unknown) =>
      error instanceof LocalFileInputError &&
      error.code === 'file_read_failed' &&
      !error.message.includes('private browser detail'),
  )
})
