use html5gum::{DefaultEmitter, HtmlString, Span, Token, Tokenizer};
use scraper::Html;

use crate::{
    BomKind, EvidenceKind, EvidenceSource, ProvenanceEvidence, SourceByteRange, SourceEncoding,
    TextDecodeResult, ingestion::decode_text_bytes, scan_text,
};

use super::model::{
    DecodedSourceRange, HtmlAnalysis, HtmlAnalysisError, HtmlAttributeRecord, HtmlCommentRecord,
    HtmlDocumentSummary, HtmlElementRecord, HtmlEncodingDeclaration, HtmlEncodingDeclarationKind,
    HtmlEncodingEvidence, HtmlExternalReference, HtmlFinding, HtmlFindingKind, HtmlFindingSeverity,
    HtmlMetaDeclaration, HtmlMetadata, HtmlParserLayer, HtmlParserRecovery, HtmlReferenceKind,
    HtmlResourceLimit, HtmlSourceRange, HtmlTextChannels, HtmlTextClassification, HtmlTextSegment,
    MAX_HTML_ATTRIBUTE_VALUE_BYTES, MAX_HTML_ATTRIBUTES, MAX_HTML_COMMENT_BYTES, MAX_HTML_COMMENTS,
    MAX_HTML_NESTING_DEPTH, MAX_HTML_PARSE_RECOVERIES, MAX_HTML_SOURCE_BYTES,
    MAX_HTML_SOURCE_ELEMENTS, MAX_HTML_TEXT_TOKEN_BYTES, MAX_HTML_TOKENS,
};

#[derive(Debug)]
struct SourceTextObservation {
    decoded_text: String,
    source_text: String,
    source_range: HtmlSourceRange,
}

#[derive(Debug)]
struct SourceInspection {
    elements: Vec<HtmlElementRecord>,
    comments: Vec<HtmlCommentRecord>,
    doctype: Option<super::model::HtmlDoctypeRecord>,
    encoding_declarations: Vec<HtmlEncodingDeclaration>,
    text_observations: Vec<SourceTextObservation>,
    recoveries: Vec<HtmlParserRecovery>,
    attribute_count: usize,
}

/// Strictly decodes and forensically analyzes untrusted HTML bytes.
///
/// This function performs no filesystem, rendering, script execution, URL
/// resolution, or network access. HTML5 tree construction and token-level
/// source evidence remain separate so reconstructed nodes never receive
/// invented source offsets.
pub fn analyze_html_bytes(bytes: &[u8]) -> Result<HtmlAnalysis, HtmlAnalysisError> {
    if bytes.len() > MAX_HTML_SOURCE_BYTES {
        return Err(HtmlAnalysisError::ResourceLimitExceeded(
            HtmlResourceLimit::SourceBytes,
        ));
    }

    let decode = decode_text_bytes(bytes)?;
    let mut source = inspect_source(&decode)?;
    let encoding = inspect_encoding(&decode, &source.encoding_declarations)?;
    let document = Html::parse_document(&decode.decoded_text);

    if source.recoveries.len() + document.errors.len() > MAX_HTML_PARSE_RECOVERIES {
        return Err(HtmlAnalysisError::ResourceLimitExceeded(
            HtmlResourceLimit::ParseRecoveries,
        ));
    }

    let tokenizer_recoveries = source.recoveries.len();
    for message in &document.errors {
        source.recoveries.push(HtmlParserRecovery {
            sequence: source.recoveries.len() + 1,
            layer: HtmlParserLayer::TreeBuilder,
            message: message.to_string(),
            source_range: None,
        });
    }

    let mut metadata = metadata_from_source(&source.elements);
    let mut parsed_elements = 0;
    let mut parsed_nodes = 0;
    let mut html_present = false;
    let mut head_present = false;
    let mut body_present = false;
    let mut title_present = false;
    let mut parsed_attribute_count = 0;

    for node in document.tree.root().descendants() {
        parsed_nodes += 1;
        if parsed_nodes > MAX_HTML_TOKENS {
            return Err(HtmlAnalysisError::ResourceLimitExceeded(
                HtmlResourceLimit::ParsedNodes,
            ));
        }
        if node.ancestors().take(MAX_HTML_NESTING_DEPTH + 2).count() > MAX_HTML_NESTING_DEPTH + 1 {
            return Err(HtmlAnalysisError::ResourceLimitExceeded(
                HtmlResourceLimit::NestingDepth,
            ));
        }

        let Some(element) = node.value().as_element() else {
            continue;
        };
        parsed_elements += 1;
        parsed_attribute_count += element.attrs().count();
        if parsed_attribute_count > MAX_HTML_ATTRIBUTES {
            return Err(HtmlAnalysisError::ResourceLimitExceeded(
                HtmlResourceLimit::Attributes,
            ));
        }

        match element.name() {
            "html" => {
                html_present = true;
                if metadata.lang.is_none() {
                    metadata.lang = element.attr("lang").map(str::to_owned);
                }
                if metadata.dir.is_none() {
                    metadata.dir = element.attr("dir").map(str::to_owned);
                }
            }
            "head" => head_present = true,
            "body" => body_present = true,
            "title" => title_present = true,
            _ => {}
        }
    }

    let text_channels_and_segments = extract_text(&document, &decode, source.text_observations)?;
    let mut text_channels = text_channels_and_segments.channels;
    let text_segments = text_channels_and_segments.segments;
    metadata.title = title_from_segments(&text_segments, title_present);
    text_channels.metadata_text = metadata_channel(&metadata, &text_segments);

    let external_references = external_references(&source.elements);
    let encoding_conflict_range = if encoding.declaration_conflict {
        encoding
            .declarations
            .iter()
            .find(|declaration| {
                !declared_encoding_matches(&declaration.normalized_label, decode.encoding)
            })
            .map(|declaration| declaration.source_range)
    } else {
        None
    };
    let mut findings = structural_findings(
        &source.elements,
        &external_references,
        encoding_conflict_range,
    );
    if !source.recoveries.is_empty() {
        findings.push(HtmlFinding {
            sequence: findings.len() + 1,
            kind: HtmlFindingKind::ParserRecoveryObserved,
            severity: HtmlFindingSeverity::Informational,
            element_sequence: None,
            attribute: None,
            description: "The HTML parser reported one or more recoverable source conditions.",
            source_range: source
                .recoveries
                .iter()
                .find_map(|recovery| recovery.source_range),
        });
    }

    let element_count = |name: &str| {
        source
            .elements
            .iter()
            .filter(|element| element.name == name)
            .count()
    };
    let summary = HtmlDocumentSummary {
        doctype: source.doctype,
        quirks_mode: format!("{:?}", document.quirks_mode),
        parsed_elements,
        source_start_tags: source.elements.len(),
        attributes: source.attribute_count,
        text_segments: text_segments.len(),
        comments: source.comments.len(),
        script_blocks: element_count("script"),
        style_blocks: element_count("style"),
        noscript_blocks: element_count("noscript"),
        template_blocks: element_count("template"),
        external_references: external_references.len(),
        tokenizer_recoveries,
        tree_builder_recoveries: document.errors.len(),
        html_present,
        head_present,
        body_present,
        title_present,
        meta_elements: element_count("meta"),
        link_elements: element_count("link"),
    };
    let scan = scan_text(&text_channels.document_text);
    let provenance = html_provenance(&metadata, &encoding);

    Ok(HtmlAnalysis {
        raw_byte_length: bytes.len(),
        decode,
        encoding,
        summary,
        metadata,
        provenance,
        elements: source.elements,
        text_segments,
        comments: source.comments,
        external_references,
        findings,
        parser_recoveries: source.recoveries,
        text_channels,
        scan,
    })
}

fn html_provenance(
    metadata: &HtmlMetadata,
    encoding: &HtmlEncodingEvidence,
) -> Vec<ProvenanceEvidence> {
    let mut evidence = Vec::new();
    if let Some(title) = &metadata.title {
        evidence.push(ProvenanceEvidence::declared(
            EvidenceKind::MetadataDeclaration,
            EvidenceSource::HtmlMetadata,
            "title",
            title,
        ));
    }
    if let Some(lang) = &metadata.lang {
        evidence.push(ProvenanceEvidence::declared(
            EvidenceKind::MetadataDeclaration,
            EvidenceSource::HtmlMetadata,
            "lang",
            lang,
        ));
    }
    if let Some(dir) = &metadata.dir {
        evidence.push(ProvenanceEvidence::declared(
            EvidenceKind::MetadataDeclaration,
            EvidenceSource::HtmlMetadata,
            "dir",
            dir,
        ));
    }
    for declaration in &metadata.meta_declarations {
        let subject = declaration
            .name
            .as_deref()
            .or(declaration.property.as_deref())
            .unwrap_or("meta")
            .to_owned();
        let kind = if subject.eq_ignore_ascii_case("generator") {
            EvidenceKind::GeneratorDeclaration
        } else {
            EvidenceKind::MetadataDeclaration
        };
        let value = declaration
            .content
            .as_deref()
            .or(declaration.charset.as_deref())
            .unwrap_or("");
        evidence.push(ProvenanceEvidence::declared(
            kind,
            if matches!(kind, EvidenceKind::GeneratorDeclaration) {
                EvidenceSource::HtmlGenerator
            } else {
                EvidenceSource::HtmlMetadata
            },
            format!("meta[{}].{subject}", declaration.sequence),
            value,
        ));
    }
    for (index, value) in metadata.canonical_links.iter().enumerate() {
        evidence.push(ProvenanceEvidence::declared(
            EvidenceKind::CanonicalDeclaration,
            EvidenceSource::HtmlCanonicalLink,
            format!("link[{}].canonical", index + 1),
            value,
        ));
    }
    for (index, value) in metadata.base_hrefs.iter().enumerate() {
        evidence.push(ProvenanceEvidence::declared(
            EvidenceKind::MetadataDeclaration,
            EvidenceSource::HtmlMetadata,
            format!("base[{}].href", index + 1),
            value,
        ));
    }
    for declaration in &encoding.declarations {
        evidence.push(ProvenanceEvidence::declared(
            EvidenceKind::EncodingDeclaration,
            EvidenceSource::HtmlEncoding,
            format!("encoding[{}]", declaration.sequence),
            &declaration.declared_label,
        ));
    }
    if encoding.declaration_conflict {
        evidence.push(ProvenanceEvidence::conflict(
            EvidenceKind::EncodingDeclaration,
            EvidenceSource::HtmlEncoding,
            "encoding",
            "Declared HTML encodings disagree with the selected decoder.",
        ));
    }
    evidence
}

fn inspect_source(decode: &TextDecodeResult) -> Result<SourceInspection, HtmlAnalysisError> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let tokenizer = Tokenizer::new_with_emitter(decode.decoded_text.as_str(), emitter);
    let mut inspection = SourceInspection {
        elements: Vec::new(),
        comments: Vec::new(),
        doctype: None,
        encoding_declarations: Vec::new(),
        text_observations: Vec::new(),
        recoveries: Vec::new(),
        attribute_count: 0,
    };
    let mut token_count = 0;

    for token in tokenizer {
        token_count += 1;
        if token_count > MAX_HTML_TOKENS {
            return Err(HtmlAnalysisError::ResourceLimitExceeded(
                HtmlResourceLimit::Tokens,
            ));
        }
        let token = token.map_err(|_| HtmlAnalysisError::ParserFailure)?;

        match token {
            Token::StartTag(tag) => {
                if inspection.elements.len() >= MAX_HTML_SOURCE_ELEMENTS {
                    return Err(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::Elements,
                    ));
                }
                inspection.attribute_count = inspection
                    .attribute_count
                    .checked_add(tag.attributes.len())
                    .ok_or(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::Attributes,
                    ))?;
                if inspection.attribute_count > MAX_HTML_ATTRIBUTES {
                    return Err(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::Attributes,
                    ));
                }

                let mut attributes = Vec::with_capacity(tag.attributes.len());
                for (name, value) in tag.attributes {
                    if value.value.len() > MAX_HTML_ATTRIBUTE_VALUE_BYTES {
                        return Err(HtmlAnalysisError::ResourceLimitExceeded(
                            HtmlResourceLimit::AttributeValueBytes,
                        ));
                    }
                    attributes.push(HtmlAttributeRecord {
                        name: checked_string(&name)?,
                        value: checked_string(&value.value)?,
                        source_range: source_range(decode, value.span)?,
                    });
                }

                let element = HtmlElementRecord {
                    sequence: inspection.elements.len() + 1,
                    name: checked_string(&tag.name)?,
                    attributes,
                    self_closing: tag.self_closing,
                    source_range: source_range(decode, tag.span)?,
                };
                if let Some(declaration) = encoding_declaration_from_element(
                    &element,
                    inspection.encoding_declarations.len() + 1,
                ) {
                    inspection.encoding_declarations.push(declaration);
                }
                inspection.elements.push(element);
            }
            Token::EndTag(_) => {}
            Token::String(text) => {
                if text.value.len() > MAX_HTML_TEXT_TOKEN_BYTES {
                    return Err(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::TextTokenBytes,
                    ));
                }
                let range = source_range(decode, text.span)?;
                let source_text = decoded_source_slice(decode, range)?.to_owned();
                inspection.text_observations.push(SourceTextObservation {
                    decoded_text: checked_string(&text.value)?,
                    source_text,
                    source_range: range,
                });
            }
            Token::Comment(comment) => {
                if inspection.comments.len() >= MAX_HTML_COMMENTS {
                    return Err(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::Comments,
                    ));
                }
                if comment.value.len() > MAX_HTML_COMMENT_BYTES {
                    return Err(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::CommentBytes,
                    ));
                }
                inspection.comments.push(HtmlCommentRecord {
                    sequence: inspection.comments.len() + 1,
                    text: checked_string(&comment.value)?,
                    source_range: source_range(decode, comment.span)?,
                });
            }
            Token::Doctype(doctype) => {
                if inspection.doctype.is_none() {
                    inspection.doctype = Some(super::model::HtmlDoctypeRecord {
                        name: checked_string(&doctype.value.name)?,
                        public_identifier: doctype
                            .value
                            .public_identifier
                            .as_ref()
                            .map(checked_string)
                            .transpose()?,
                        system_identifier: doctype
                            .value
                            .system_identifier
                            .as_ref()
                            .map(checked_string)
                            .transpose()?,
                        force_quirks: doctype.value.force_quirks,
                        source_range: source_range(decode, doctype.span)?,
                    });
                }
            }
            Token::Error(error) => {
                if inspection.recoveries.len() >= MAX_HTML_PARSE_RECOVERIES {
                    return Err(HtmlAnalysisError::ResourceLimitExceeded(
                        HtmlResourceLimit::ParseRecoveries,
                    ));
                }
                inspection.recoveries.push(HtmlParserRecovery {
                    sequence: inspection.recoveries.len() + 1,
                    layer: HtmlParserLayer::Tokenizer,
                    message: format!("{:?}", error.value),
                    source_range: Some(source_range(decode, error.span)?),
                });
            }
        }
    }

    Ok(inspection)
}

fn inspect_encoding(
    decode: &TextDecodeResult,
    declarations: &[HtmlEncodingDeclaration],
) -> Result<HtmlEncodingEvidence, HtmlAnalysisError> {
    let selected = decode.encoding;
    let has_bom = decode.bom != BomKind::None;
    let mut declaration_conflict = false;

    for declaration in declarations {
        let declared = declared_source_encoding(&declaration.normalized_label);
        if !declared_encoding_matches(&declaration.normalized_label, selected) {
            declaration_conflict = true;
        }
        if !has_bom && declared != Some(SourceEncoding::Utf8) {
            return Err(HtmlAnalysisError::UnsupportedDeclaredEncoding);
        }
    }

    Ok(HtmlEncodingEvidence {
        declarations: declarations.to_vec(),
        bom_precedence_applied: has_bom && !declarations.is_empty(),
        declaration_conflict,
    })
}

fn encoding_declaration_from_element(
    element: &HtmlElementRecord,
    sequence: usize,
) -> Option<HtmlEncodingDeclaration> {
    if element.name != "meta"
        || element
            .source_range
            .raw_source_byte_range
            .is_none_or(|range| range.start >= 1024)
    {
        return None;
    }

    if let Some(label) = attribute(element, "charset") {
        return Some(HtmlEncodingDeclaration {
            sequence,
            kind: HtmlEncodingDeclarationKind::MetaCharset,
            declared_label: label.to_owned(),
            normalized_label: normalize_encoding_label(label),
            source_range: element.source_range,
        });
    }

    let http_equiv = attribute(element, "http-equiv")?;
    if !http_equiv.eq_ignore_ascii_case("content-type") {
        return None;
    }
    let content = attribute(element, "content")?;
    let label = content_type_charset(content)?;

    Some(HtmlEncodingDeclaration {
        sequence,
        kind: HtmlEncodingDeclarationKind::HttpEquivContentType,
        declared_label: label.to_owned(),
        normalized_label: normalize_encoding_label(label),
        source_range: element.source_range,
    })
}

fn content_type_charset(content: &str) -> Option<&str> {
    content.split(';').find_map(|part| {
        let (name, value) = part.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case("charset")
            .then(|| value.trim().trim_matches(['\'', '"']))
    })
}

fn normalize_encoding_label(label: &str) -> String {
    label.trim().trim_matches(['\'', '"']).to_ascii_lowercase()
}

fn declared_source_encoding(label: &str) -> Option<SourceEncoding> {
    match label {
        "utf-8" | "utf8" | "unicode-1-1-utf-8" => Some(SourceEncoding::Utf8),
        "utf-16" | "utf-16le" => Some(SourceEncoding::Utf16LittleEndian),
        "utf-16be" => Some(SourceEncoding::Utf16BigEndian),
        _ => None,
    }
}

fn declared_encoding_matches(label: &str, selected: SourceEncoding) -> bool {
    match label {
        "utf-16" => matches!(
            selected,
            SourceEncoding::Utf16LittleEndian | SourceEncoding::Utf16BigEndian
        ),
        _ => declared_source_encoding(label) == Some(selected),
    }
}

fn metadata_from_source(elements: &[HtmlElementRecord]) -> HtmlMetadata {
    let mut metadata = HtmlMetadata::default();

    for element in elements {
        match element.name.as_str() {
            "meta" => metadata.meta_declarations.push(HtmlMetaDeclaration {
                sequence: metadata.meta_declarations.len() + 1,
                charset: attribute(element, "charset").map(str::to_owned),
                name: attribute(element, "name").map(str::to_owned),
                property: attribute(element, "property").map(str::to_owned),
                content: attribute(element, "content").map(str::to_owned),
                source_range: element.source_range,
            }),
            "link" => {
                if attribute(element, "rel").is_some_and(|value| {
                    value
                        .split_ascii_whitespace()
                        .any(|token| token.eq_ignore_ascii_case("canonical"))
                }) && let Some(href) = attribute(element, "href")
                {
                    metadata.canonical_links.push(href.to_owned());
                }
            }
            "base" => {
                if let Some(href) = attribute(element, "href") {
                    metadata.base_hrefs.push(href.to_owned());
                }
            }
            _ => {}
        }
    }

    metadata
}

struct TextExtraction {
    segments: Vec<HtmlTextSegment>,
    channels: HtmlTextChannels,
}

fn extract_text(
    document: &Html,
    decode: &TextDecodeResult,
    observations: Vec<SourceTextObservation>,
) -> Result<TextExtraction, HtmlAnalysisError> {
    let mut segments = Vec::new();
    let mut channels = HtmlTextChannels::default();
    let mut observation_cursor = 0;
    let mut last_block_id = None;
    let mut pending_break = false;

    for node in document.tree.root().descendants() {
        if let Some(element) = node.value().as_element() {
            let excluded_context = node.ancestors().any(|ancestor| {
                ancestor
                    .value()
                    .as_element()
                    .is_some_and(|ancestor_element| {
                        matches!(
                            ancestor_element.name(),
                            "script" | "style" | "template" | "noscript" | "title" | "head"
                        )
                    })
            });
            if element.name() == "br" && !excluded_context {
                pending_break = true;
            }
            continue;
        }

        let Some(text) = node.value().as_text() else {
            continue;
        };
        let value: &str = text;
        let mut classification = HtmlTextClassification::VisibleContentCandidate;
        let mut in_head = false;
        for ancestor in node.ancestors() {
            let Some(element) = ancestor.value().as_element() else {
                continue;
            };
            classification = match element.name() {
                "script" => HtmlTextClassification::ScriptContent,
                "style" => HtmlTextClassification::StyleContent,
                "template" => HtmlTextClassification::TemplateContent,
                "noscript" => HtmlTextClassification::NoscriptContent,
                "title" => HtmlTextClassification::Title,
                "head" => {
                    in_head = true;
                    continue;
                }
                _ => continue,
            };
            break;
        }
        if classification == HtmlTextClassification::VisibleContentCandidate && in_head {
            classification = HtmlTextClassification::Metadata;
        }
        let context_element = node
            .parent()
            .and_then(|parent| parent.value().as_element())
            .map_or_else(
                || "document".to_owned(),
                |element| element.name().to_owned(),
            );
        let (source_text, source_range) =
            correlate_source_text(value, decode, &observations, &mut observation_cursor)?;

        segments.push(HtmlTextSegment {
            sequence: segments.len() + 1,
            text: value.to_owned(),
            context_element,
            classification,
            source_text,
            source_range,
        });

        match classification {
            HtmlTextClassification::VisibleContentCandidate => {
                let current_block_id = node
                    .ancestors()
                    .find(|ancestor| {
                        ancestor
                            .value()
                            .as_element()
                            .is_some_and(|element| is_block_element(element.name()))
                    })
                    .map(|ancestor| ancestor.id());
                let block_changed = last_block_id.is_some()
                    && current_block_id.is_some()
                    && last_block_id != current_block_id;
                if pending_break || block_changed {
                    append_separator_if_needed(&mut channels.document_text, value);
                }
                channels.document_text.push_str(value);
                last_block_id = current_block_id;
                pending_break = false;
            }
            HtmlTextClassification::ScriptContent => {
                append_channel_text(&mut channels.active_content_text, value)
            }
            HtmlTextClassification::StyleContent => {
                append_channel_text(&mut channels.style_text, value)
            }
            HtmlTextClassification::TemplateContent => {
                append_channel_text(&mut channels.template_text, value)
            }
            HtmlTextClassification::NoscriptContent => {
                append_channel_text(&mut channels.noscript_text, value)
            }
            HtmlTextClassification::Title | HtmlTextClassification::Metadata => {}
        }
    }

    Ok(TextExtraction { segments, channels })
}

fn correlate_source_text(
    text: &str,
    decode: &TextDecodeResult,
    observations: &[SourceTextObservation],
    cursor: &mut usize,
) -> Result<(Option<String>, Option<HtmlSourceRange>), HtmlAnalysisError> {
    if text.is_empty() || *cursor >= observations.len() {
        return Ok((None, None));
    }

    let start = *cursor;
    let mut decoded = String::new();
    let mut source = String::new();
    let mut end = start;
    let mut previous_end = None;

    while let Some(observation) = observations.get(end) {
        if previous_end
            .is_some_and(|value| value != observation.source_range.decoded_utf8_byte_range.start)
        {
            break;
        }
        decoded.push_str(&observation.decoded_text);
        source.push_str(&observation.source_text);
        previous_end = Some(observation.source_range.decoded_utf8_byte_range.end);
        end += 1;

        if decoded == text {
            let decoded_range = DecodedSourceRange {
                start: observations[start]
                    .source_range
                    .decoded_utf8_byte_range
                    .start,
                end: observations[end - 1]
                    .source_range
                    .decoded_utf8_byte_range
                    .end,
            };
            let combined_range = source_range_from_decoded(decode, decoded_range)?;
            *cursor = end;
            return Ok((Some(source), Some(combined_range)));
        }
        if decoded.len() >= text.len() {
            break;
        }
    }

    Ok((None, None))
}

fn title_from_segments(segments: &[HtmlTextSegment], title_present: bool) -> Option<String> {
    if !title_present {
        return None;
    }
    Some(
        segments
            .iter()
            .filter(|segment| segment.classification == HtmlTextClassification::Title)
            .map(|segment| segment.text.as_str())
            .collect(),
    )
}

fn metadata_channel(metadata: &HtmlMetadata, segments: &[HtmlTextSegment]) -> String {
    let mut channel = String::new();
    if let Some(title) = &metadata.title {
        append_channel_text(&mut channel, title);
    }
    for segment in segments
        .iter()
        .filter(|segment| segment.classification == HtmlTextClassification::Metadata)
    {
        append_channel_text(&mut channel, &segment.text);
    }
    for declaration in &metadata.meta_declarations {
        if let Some(content) = &declaration.content {
            append_channel_text(&mut channel, content);
        }
    }
    channel
}

fn external_references(elements: &[HtmlElementRecord]) -> Vec<HtmlExternalReference> {
    let mut references = Vec::new();

    for element in elements {
        for attribute in &element.attributes {
            if !is_reference_attribute(&element.name, &attribute.name) {
                continue;
            }
            references.push(HtmlExternalReference {
                sequence: references.len() + 1,
                element: element.name.clone(),
                attribute: attribute.name.clone(),
                value: attribute.value.clone(),
                kind: if attribute.name == "srcset" {
                    HtmlReferenceKind::Other
                } else {
                    classify_reference(&attribute.value)
                },
                source_range: attribute.source_range,
            });
        }
    }

    references
}

fn structural_findings(
    elements: &[HtmlElementRecord],
    references: &[HtmlExternalReference],
    encoding_conflict_range: Option<HtmlSourceRange>,
) -> Vec<HtmlFinding> {
    let mut findings = Vec::new();

    if let Some(source_range) = encoding_conflict_range {
        findings.push(HtmlFinding {
            sequence: findings.len() + 1,
            kind: HtmlFindingKind::EncodingDeclarationConflict,
            severity: HtmlFindingSeverity::Caution,
            element_sequence: None,
            attribute: None,
            description: "A source charset declaration conflicts with the BOM-selected encoding.",
            source_range: Some(source_range),
        });
    }

    for element in elements {
        if element.name == "script" {
            findings.push(HtmlFinding {
                sequence: findings.len() + 1,
                kind: HtmlFindingKind::ActiveScriptPresent,
                severity: HtmlFindingSeverity::Caution,
                element_sequence: Some(element.sequence),
                attribute: None,
                description: "A script element is present as inert source structure.",
                source_range: Some(element.source_range),
            });
        }

        for attribute in &element.attributes {
            if is_event_handler_attribute(&attribute.name) {
                findings.push(HtmlFinding {
                    sequence: findings.len() + 1,
                    kind: HtmlFindingKind::InlineEventHandlerPresent,
                    severity: HtmlFindingSeverity::Caution,
                    element_sequence: Some(element.sequence),
                    attribute: Some(attribute.name.clone()),
                    description: "A recognized inline HTML event-handler attribute is present as inert source data.",
                    source_range: Some(attribute.source_range),
                });
            }
            if is_explicit_hidden_marker(attribute) {
                findings.push(HtmlFinding {
                    sequence: findings.len() + 1,
                    kind: HtmlFindingKind::ExplicitHiddenMarker,
                    severity: HtmlFindingSeverity::Informational,
                    element_sequence: Some(element.sequence),
                    attribute: Some(attribute.name.clone()),
                    description: "An explicit hidden marker is present; no CSS cascade or rendered visibility was inferred.",
                    source_range: Some(attribute.source_range),
                });
            }
        }
    }

    for reference in references {
        findings.push(HtmlFinding {
            sequence: findings.len() + 1,
            kind: HtmlFindingKind::ExternalReferencePresent,
            severity: HtmlFindingSeverity::Informational,
            element_sequence: None,
            attribute: Some(reference.attribute.clone()),
            description: "A reference-bearing HTML attribute was observed without resolving or fetching it.",
            source_range: Some(reference.source_range),
        });
    }

    findings
}

fn attribute<'a>(element: &'a HtmlElementRecord, name: &str) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.as_str())
}

fn is_reference_attribute(element: &str, attribute: &str) -> bool {
    match attribute {
        "href" => matches!(element, "a" | "area" | "base" | "link"),
        "src" => matches!(
            element,
            "audio"
                | "embed"
                | "iframe"
                | "img"
                | "input"
                | "script"
                | "source"
                | "track"
                | "video"
        ),
        "srcset" => matches!(element, "img" | "source"),
        "action" => element == "form",
        "poster" => element == "video",
        "data" => element == "object",
        "formaction" => matches!(element, "button" | "input"),
        _ => false,
    }
}

fn classify_reference(value: &str) -> HtmlReferenceKind {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return HtmlReferenceKind::Other;
    }
    if trimmed.starts_with('#') {
        return HtmlReferenceKind::Fragment;
    }
    if trimmed
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("data:"))
    {
        return HtmlReferenceKind::DataUrl;
    }
    if trimmed.starts_with("//") || has_uri_scheme(trimmed) {
        return HtmlReferenceKind::Absolute;
    }
    if trimmed.chars().any(char::is_control) {
        HtmlReferenceKind::Other
    } else {
        HtmlReferenceKind::Relative
    }
}

fn has_uri_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

fn is_event_handler_attribute(name: &str) -> bool {
    matches!(
        name,
        "onabort"
            | "onauxclick"
            | "onbeforeinput"
            | "onbeforetoggle"
            | "onblur"
            | "oncancel"
            | "oncanplay"
            | "onchange"
            | "onclick"
            | "onclose"
            | "oncontextmenu"
            | "oncopy"
            | "oncut"
            | "ondblclick"
            | "ondrag"
            | "ondrop"
            | "onerror"
            | "onfocus"
            | "onformdata"
            | "oninput"
            | "oninvalid"
            | "onkeydown"
            | "onkeypress"
            | "onkeyup"
            | "onload"
            | "onmousedown"
            | "onmouseenter"
            | "onmouseleave"
            | "onmousemove"
            | "onmouseout"
            | "onmouseover"
            | "onmouseup"
            | "onpaste"
            | "onpointerdown"
            | "onpointermove"
            | "onpointerup"
            | "onreset"
            | "onresize"
            | "onscroll"
            | "onsubmit"
            | "ontoggle"
            | "onwheel"
    )
}

fn is_explicit_hidden_marker(attribute: &HtmlAttributeRecord) -> bool {
    match attribute.name.as_str() {
        "hidden" => true,
        "aria-hidden" => attribute.value.trim().eq_ignore_ascii_case("true"),
        "style" => has_trivial_hidden_style(&attribute.value),
        _ => false,
    }
}

fn has_trivial_hidden_style(style: &str) -> bool {
    style.split(';').any(|declaration| {
        let Some((property, value)) = declaration.split_once(':') else {
            return false;
        };
        let property = property.trim();
        let value = value.trim();
        (property.eq_ignore_ascii_case("display") && value.eq_ignore_ascii_case("none"))
            || (property.eq_ignore_ascii_case("visibility") && value.eq_ignore_ascii_case("hidden"))
    })
}

fn is_block_element(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tr"
            | "ul"
    )
}

fn append_separator_if_needed(output: &mut String, next: &str) {
    if !output.is_empty()
        && !output.chars().next_back().is_some_and(char::is_whitespace)
        && !next.chars().next().is_some_and(char::is_whitespace)
    {
        output.push('\n');
    }
}

fn append_channel_text(output: &mut String, value: &str) {
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(value);
}

fn checked_string(value: &HtmlString) -> Result<String, HtmlAnalysisError> {
    String::from_utf8(value.as_ref().to_vec()).map_err(|_| HtmlAnalysisError::ParserFailure)
}

fn source_range(
    decode: &TextDecodeResult,
    span: Span<usize>,
) -> Result<HtmlSourceRange, HtmlAnalysisError> {
    source_range_from_decoded(
        decode,
        DecodedSourceRange {
            start: span.start,
            end: span.end,
        },
    )
}

fn source_range_from_decoded(
    decode: &TextDecodeResult,
    decoded_range: DecodedSourceRange,
) -> Result<HtmlSourceRange, HtmlAnalysisError> {
    if decoded_range.start > decoded_range.end
        || decoded_range.end > decode.decoded_text.len()
        || !decode.decoded_text.is_char_boundary(decoded_range.start)
        || !decode.decoded_text.is_char_boundary(decoded_range.end)
    {
        return Err(HtmlAnalysisError::ParserFailure);
    }

    let raw_start = raw_boundary(decode, decoded_range.start);
    let raw_end = raw_boundary(decode, decoded_range.end);
    let raw_source_byte_range = raw_start
        .zip(raw_end)
        .and_then(|(start, end)| (start <= end).then_some(SourceByteRange { start, end }));

    Ok(HtmlSourceRange {
        decoded_utf8_byte_range: decoded_range,
        raw_source_byte_range,
    })
}

fn raw_boundary(decode: &TextDecodeResult, decoded_offset: usize) -> Option<usize> {
    if let Some(mapping) = decode
        .source_mappings
        .iter()
        .find(|mapping| mapping.decoded_utf8_byte_offset == decoded_offset)
    {
        return Some(mapping.source_byte_range.start);
    }
    if decoded_offset == decode.decoded_text.len() {
        return decode
            .source_mappings
            .last()
            .map(|mapping| mapping.source_byte_range.end)
            .or(match decode.bom {
                BomKind::None => Some(0),
                BomKind::Utf8 => Some(3),
                BomKind::Utf16LittleEndian | BomKind::Utf16BigEndian => Some(2),
            });
    }
    None
}

fn decoded_source_slice(
    decode: &TextDecodeResult,
    range: HtmlSourceRange,
) -> Result<&str, HtmlAnalysisError> {
    decode
        .decoded_text
        .get(range.decoded_utf8_byte_range.start..range.decoded_utf8_byte_range.end)
        .ok_or(HtmlAnalysisError::ParserFailure)
}
