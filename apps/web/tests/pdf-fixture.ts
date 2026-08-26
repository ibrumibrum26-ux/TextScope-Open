import { Buffer } from 'node:buffer'

export interface PdfFixtureOptions {
  pages?: string[]
  unicode?: boolean
  metadata?: boolean
  xmp?: boolean
  javascript?: boolean
  uri?: boolean
  launch?: boolean
  submitForm?: boolean
  attachment?: boolean
  form?: boolean
  signature?: boolean
  linearized?: boolean
}

function reference(id: number): string {
  return `${id} 0 R`
}

function pdfTextString(text: string): string {
  const winAnsi: Record<string, number> = {
    'á': 0xe1, 'é': 0xe9, 'í': 0xed, 'ó': 0xf3, 'ú': 0xfa,
    'ã': 0xe3, 'õ': 0xf5, 'ç': 0xe7, 'Á': 0xc1, 'É': 0xc9,
    'Í': 0xcd, 'Ó': 0xd3, 'Ú': 0xda, 'Ã': 0xc3, 'Õ': 0xd5,
    'Ç': 0xc7, '\u00a0': 0xa0,
  }
  const bytes: number[] = []
  for (const character of text) {
    const code = character.codePointAt(0)!
    const encoded = code < 0x80 ? code : winAnsi[character]
    if (encoded === undefined) throw new Error(`Fixture text is not representable in WinAnsi: ${character}`)
    bytes.push(encoded)
  }
  return `<${Buffer.from(bytes).toString('hex')}>`
}

function pdfUnicodeTextString(text: string): string {
  const bytes: string[] = []
  for (const character of text) {
    const codePoint = character.codePointAt(0)!
    // The fixture uses one stable CID for 😀 so the Rust PDF adapter can
    // exercise a real ToUnicode surrogate-pair mapping in the browser gate.
    const cid = codePoint === 0x1f600 ? 0x005f : codePoint
    if (cid > 0xffff) throw new Error(`Fixture Unicode code point is not representable: ${character}`)
    bytes.push(cid.toString(16).padStart(4, '0'))
  }
  return `<${bytes.join('')}>`
}

export function createPdf(options: PdfFixtureOptions = {}): Buffer {
  const pages = options.pages ?? ['Hello PDF']
  const objects: string[] = []
  const add = (body: string): number => {
    objects.push(body)
    return objects.length
  }
  const catalogId = add('')
  const pagesId = add('')
  const unicodeCmap = `/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n3 beginbfchar\n<0041> <0041>\n<005F> <D83DDE00>\n<0042> <0042>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend`
  const unicodeCmapId = options.unicode
    ? add(`<< /Length ${Buffer.byteLength(unicodeCmap, 'utf8')} >>\nstream\n${unicodeCmap}\nendstream`)
    : null
  const fontId = options.unicode
    ? add(`<< /Type /Font /Subtype /Type0 /BaseFont /Identity /Encoding /Identity-H /ToUnicode ${reference(unicodeCmapId!)} >>`)
    : add('<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>')
  const pageIds: number[] = []
  const catalogParts = [`/Type /Catalog`, `/Pages ${reference(pagesId)}`]

  for (const text of pages) {
    const encodedText = options.unicode ? pdfUnicodeTextString(text) : pdfTextString(text)
    const content = text === ''
      ? 'q 1 0 0 1 0 0 cm Q'
      : `BT /F1 12 Tf 72 720 Td ${encodedText} Tj ET`
    const contentId = add(`<< /Length ${Buffer.byteLength(content, 'utf8')} >>\nstream\n${content}\nendstream`)
    pageIds.push(add(''))
    objects[pageIds.at(-1)! - 1] = `<< /Type /Page /Parent ${reference(pagesId)} /MediaBox [0 0 612 792] /Resources << /Font << /F1 ${reference(fontId)} >> >> /Contents ${reference(contentId)}`
      + ' >>'
  }

  objects[pagesId - 1] = `<< /Type /Pages /Kids [${pageIds.map(reference).join(' ')}] /Count ${pageIds.length} >>`

  const actionRefs: number[] = []
  if (options.javascript) actionRefs.push(add('<< /S /JavaScript /JS (app.alert) >>'))
  if (options.uri) actionRefs.push(add('<< /S /URI /URI (https://example.invalid/textscope) >>'))
  if (options.launch) actionRefs.push(add('<< /S /Launch /F (evidence.exe) >>'))
  if (options.submitForm) actionRefs.push(add('<< /S /SubmitForm /F (https://example.invalid/submit) >>'))
  if (actionRefs.length > 0) {
    const annotationId = add(`<< /Type /Annot /Subtype /Link /Rect [0 0 120 30] /A ${reference(actionRefs[0]!)} >>`)
    const firstPage = pageIds[0]!
    objects[firstPage - 1] = `${objects[firstPage - 1]!.slice(0, -3)} /Annots [${reference(annotationId)}] >>`
    catalogParts.push(`/OpenAction ${reference(actionRefs[0]!)}`)
  }

  if (options.attachment) {
    const streamId = add('<< /Type /EmbeddedFile /Subtype /text#2Fplain /Params << /Size 18 >> /Length 18 >>\nstream\nembedded payload!\nendstream')
    const fileSpecId = add(`<< /Type /Filespec /F (evidence.txt) /Desc (Observed attachment) /EF << /F ${reference(streamId)} >> >>`)
    catalogParts.push(`/Names << /EmbeddedFiles << /Names [(evidence.txt) ${reference(fileSpecId)}] >> >>`)
  }

  if (options.form || options.signature) {
    let fieldRef = ''
    if (options.signature) {
      const signatureId = add('<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 10 20 30] /Contents <00> >>')
      fieldRef = reference(add(`<< /FT /Sig /T (signature) /V ${reference(signatureId)} >>`))
    } else {
      fieldRef = reference(add('<< /FT /Tx /T (field) >>'))
    }
    const formId = add(`<< /Fields [${fieldRef}] /NeedAppearances true${options.form ? ' /XFA (not executed)' : ''} >>`)
    catalogParts.push(`/AcroForm ${reference(formId)}`)
  }

  if (options.xmp) {
    const xmp = '<?xpacket begin=""?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description><dc:title xmlns:dc="http://purl.org/dc/elements/1.1/">XMP title</dc:title></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end="w"?>'
    const metadataId = add(`<< /Type /Metadata /Subtype /XML /Length ${Buffer.byteLength(xmp, 'utf8')} >>\nstream\n${xmp}\nendstream`)
    catalogParts.push(`/Metadata ${reference(metadataId)}`)
  }

  if (options.linearized) catalogParts.push('/Linearized 1')
  objects[catalogId - 1] = `<< ${catalogParts.join(' ')} >>`
  if (options.metadata) {
    const infoId = add('<< /Title (Declared title) /Author (Declared author) /Producer (TextScope fixture) >>')
    return serializePdf(objects, ` /Root ${reference(catalogId)} /Info ${reference(infoId)}`)
  }
  return serializePdf(objects, ` /Root ${reference(catalogId)}`)
}

function serializePdf(objects: string[], trailerExtra: string): Buffer {
  const chunks: Buffer[] = [Buffer.from('%PDF-1.7\n%\xE2\xE3\xCF\xD3\n', 'binary')]
  const offsets = [0]
  for (const [index, body] of objects.entries()) {
    offsets.push(Buffer.concat(chunks).length)
    chunks.push(Buffer.from(`${index + 1} 0 obj\n${body}\nendobj\n`, 'utf8'))
  }
  const xrefOffset = Buffer.concat(chunks).length
  chunks.push(Buffer.from(`xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`, 'utf8'))
  for (const offset of offsets.slice(1)) chunks.push(Buffer.from(`${offset.toString().padStart(10, '0')} 00000 n \n`, 'utf8'))
  chunks.push(Buffer.from(`trailer\n<< /Size ${objects.length + 1}${trailerExtra} >>\nstartxref\n${xrefOffset}\n%%EOF\n`, 'utf8'))
  return Buffer.concat(chunks)
}
