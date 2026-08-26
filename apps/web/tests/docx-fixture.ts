import { Buffer } from 'node:buffer'

export interface DocxFixtureOptions {
  bodyXml?: string
  documentRelationships?: string
  extraEntries?: Array<[string, string | Uint8Array]>
}

interface ZipEntry {
  name: Buffer
  data: Buffer
  crc32: number
  offset: number
}

const crcTable = Array.from({ length: 256 }, (_, value) => {
  let crc = value
  for (let bit = 0; bit < 8; bit += 1) {
    crc = (crc & 1) === 1 ? 0xedb88320 ^ (crc >>> 1) : crc >>> 1
  }
  return crc >>> 0
})

function crc32(data: Buffer): number {
  let crc = 0xffffffff
  for (const byte of data) {
    crc = (crc >>> 8) ^ (crcTable[(crc ^ byte) & 0xff] ?? 0)
  }
  return (crc ^ 0xffffffff) >>> 0
}

function localHeader(entry: ZipEntry): Buffer {
  const header = Buffer.alloc(30)
  header.writeUInt32LE(0x04034b50, 0)
  header.writeUInt16LE(20, 4)
  header.writeUInt32LE(entry.crc32, 14)
  header.writeUInt32LE(entry.data.length, 18)
  header.writeUInt32LE(entry.data.length, 22)
  header.writeUInt16LE(entry.name.length, 26)
  return Buffer.concat([header, entry.name, entry.data])
}

function centralHeader(entry: ZipEntry): Buffer {
  const header = Buffer.alloc(46)
  header.writeUInt32LE(0x02014b50, 0)
  header.writeUInt16LE(20, 4)
  header.writeUInt16LE(20, 6)
  header.writeUInt32LE(entry.crc32, 16)
  header.writeUInt32LE(entry.data.length, 20)
  header.writeUInt32LE(entry.data.length, 24)
  header.writeUInt16LE(entry.name.length, 28)
  header.writeUInt32LE(entry.offset, 42)
  return Buffer.concat([header, entry.name])
}

export function createStoredZip(sourceEntries: Array<[string, string | Uint8Array]>): Buffer {
  const entries: ZipEntry[] = []
  const localRecords: Buffer[] = []
  let offset = 0

  for (const [name, sourceData] of sourceEntries) {
    const data = typeof sourceData === 'string' ? Buffer.from(sourceData, 'utf8') : Buffer.from(sourceData)
    const entry = { name: Buffer.from(name, 'utf8'), data, crc32: crc32(data), offset }
    const record = localHeader(entry)
    entries.push(entry)
    localRecords.push(record)
    offset += record.length
  }

  const centralRecords = entries.map(centralHeader)
  const centralSize = centralRecords.reduce((total, record) => total + record.length, 0)
  const end = Buffer.alloc(22)
  end.writeUInt32LE(0x06054b50, 0)
  end.writeUInt16LE(entries.length, 8)
  end.writeUInt16LE(entries.length, 10)
  end.writeUInt32LE(centralSize, 12)
  end.writeUInt32LE(offset, 16)
  return Buffer.concat([...localRecords, ...centralRecords, end])
}

export function createDocx(options: DocxFixtureOptions = {}): Buffer {
  const wordNamespace = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main'
  const body = options.bodyXml ?? '<w:p><w:r><w:t>Simple DOCX body</w:t></w:r></w:p>'
  const entries: Array<[string, string | Uint8Array]> = [
    [
      '[Content_Types].xml',
      '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
    ],
    [
      '_rels/.rels',
      '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
    ],
    [
      'word/document.xml',
      `<w:document xmlns:w="${wordNamespace}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>${body}</w:body></w:document>`,
    ],
  ]
  if (options.documentRelationships !== undefined) {
    entries.push([
      'word/_rels/document.xml.rels',
      `<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">${options.documentRelationships}</Relationships>`,
    ])
  }
  entries.push(...(options.extraEntries ?? []))
  return createStoredZip(entries)
}
