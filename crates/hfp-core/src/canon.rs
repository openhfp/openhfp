//! Canonicalization — the deterministic byte representation signatures are computed over.
//!
//! See `spec/canonicalization.md`. This module is the outcome of **Spike A**: it pins down
//! a canonical serialization that makes benign reformatting (formatter output, minification,
//! reordered attributes, CRLF/CR/LF, BOM, inter-tag whitespace, adjacent comments) collapse
//! to identical bytes, while keeping meaningful content intact.
//!
//! ## Why a custom serializer
//!
//! Neither a streaming rewriter (`lol-html`) nor html5ever's stock serializer is enough on
//! its own: the former preserves the source bytes (so reordered attributes stay different),
//! the latter preserves source attribute order. Determinism requires *active* normalization,
//! so we parse into a real DOM (html5ever) and re-serialize through the rules below.
//!
//! ## Canonical rules (v1 spike)
//!
//! 1. UTF-8 decode (hard fail otherwise); strip a leading BOM; CRLF/CR -> LF.
//! 2. Parse with the HTML5 algorithm (html5ever) — implied tags, attribute de-dup and
//!    case-folding happen here for free.
//! 3. Empty the inner content of `#hfp-data` and `#hfp-data-signature` (keep the tags).
//!    Duplicate id -> hard fail; missing required block -> hard fail.
//! 4. Serialize deterministically:
//!    - element & attribute names lowercased (done by the parser for the HTML namespace);
//!    - attributes sorted by (namespace, local name), always double-quoted;
//!    - void elements emitted without a closing tag or self-closing slash;
//!    - raw-text elements (`script`, `style`) emitted without escaping;
//!    - text escaped as `&`,`<`,`>`; attribute values as `&`,`"`;
//!    - comments dropped;
//!    - whitespace-only text nodes between elements dropped, except inside
//!      whitespace-significant elements (`pre`, `textarea`, `script`, `style`).
//!
//! Known caveat (tracked for the spec, not the spike): dropping whitespace-only text nodes
//! also drops a significant space between two *inline* elements (`<a>x</a> <a>y</a>`).
//! v1 accepts this; a later revision may scope the rule to flow content only.

use markup5ever_rcdom::{Handle, NodeData, RcDom};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

/// HTML void elements — serialized with no closing tag and no self-closing slash.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elements whose text content is whitespace-significant and emitted verbatim.
const RAW_OR_PRESERVE: &[&str] = &["pre", "textarea", "script", "style"];

/// Elements whose text children must not be HTML-escaped on output.
const RAW_TEXT: &[&str] = &["script", "style"];

/// The block ids that get their inner content emptied before hashing.
const EMPTIED_BLOCKS: &[&str] = &["hfp-data", "hfp-data-signature"];

/// Produce the canonical bytes for `raw`. See the module docs for the exact rules.
pub fn canonicalize(raw: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(raw).map_err(|_| Error::InvalidUtf8)?;

    // Step 1: strip a leading BOM, normalize line endings to LF.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let normalized = normalize_line_endings(text);

    // Step 2: parse with the HTML5 algorithm.
    use html5ever::tendril::TendrilSink;
    let dom = html5ever::parse_document(RcDom::default(), Default::default()).one(normalized);

    // Step 3: locate required blocks, fail on duplicate/missing, then empty them.
    for id in EMPTIED_BLOCKS {
        let nodes = find_by_id(&dom.document, id);
        match nodes.len() {
            0 => return Err(Error::MissingBlock(static_id(id))),
            1 => nodes[0].children.borrow_mut().clear(),
            _ => return Err(Error::DuplicateBlock(static_id(id))),
        }
    }

    // Step 4: deterministic serialization.
    let mut out = String::new();
    for child in dom.document.children.borrow().iter() {
        serialize_node(child, &mut out);
    }
    Ok(out.into_bytes())
}

/// SHA-256 of the canonical bytes, lowercase hex — the signed digest.
pub fn canonical_sha256_hex(raw: &[u8]) -> Result<String> {
    let bytes = canonicalize(raw)?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn normalize_line_endings(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Map a known block id to its `&'static str` so it can live in [`Error`].
fn static_id(id: &str) -> &'static str {
    match id {
        "hfp-data" => "hfp-data",
        "hfp-data-signature" => "hfp-data-signature",
        _ => "hfp-block",
    }
}

/// All element nodes in the tree whose `id` attribute equals `id`.
fn find_by_id(node: &Handle, id: &str) -> Vec<Handle> {
    let mut found = Vec::new();
    collect_by_id(node, id, &mut found);
    found
}

fn collect_by_id(node: &Handle, id: &str, out: &mut Vec<Handle>) {
    if let NodeData::Element { attrs, .. } = &node.data {
        for attr in attrs.borrow().iter() {
            if &*attr.name.local == "id" && &*attr.value == id {
                out.push(node.clone());
                break;
            }
        }
    }
    for child in node.children.borrow().iter() {
        collect_by_id(child, id, out);
    }
}

fn serialize_node(node: &Handle, out: &mut String) {
    match &node.data {
        NodeData::Doctype { name, .. } => {
            out.push_str("<!DOCTYPE ");
            out.push_str(name);
            out.push('>');
        }
        NodeData::Comment { .. } => {
            // Dropped: a comment is non-semantic, so an adjacent comment must stay benign.
        }
        NodeData::Text { contents } => {
            // Standalone text nodes are escaped here; raw-text element children are handled
            // by their parent in `serialize_element`, so this branch is for the rare text
            // node directly under the document (e.g. stray whitespace) — escape it.
            push_escaped_text(&contents.borrow(), out);
        }
        NodeData::Element { name, .. } => {
            serialize_element(node, &name.local, out);
        }
        _ => {}
    }
}

fn serialize_element(node: &Handle, tag: &str, out: &mut String) {
    out.push('<');
    out.push_str(tag);

    // Attributes sorted by (namespace, local name) for a stable order.
    if let NodeData::Element { attrs, .. } = &node.data {
        let mut pairs: Vec<(String, String, String)> = attrs
            .borrow()
            .iter()
            .map(|a| {
                let ns = a.name.ns.to_string();
                let local = a.name.local.to_string();
                (ns, local, a.value.to_string())
            })
            .collect();
        pairs.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        for (_, name, value) in &pairs {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");
            push_escaped_attr(value, out);
            out.push('"');
        }
    }
    out.push('>');

    if VOID_ELEMENTS.contains(&tag) {
        return; // No children, no closing tag.
    }

    let raw_text = RAW_TEXT.contains(&tag);
    let preserve_ws = RAW_OR_PRESERVE.contains(&tag);
    for child in node.children.borrow().iter() {
        if let NodeData::Text { contents } = &child.data {
            let s = contents.borrow();
            if !preserve_ws && s.trim().is_empty() {
                continue; // Drop whitespace-only text between elements.
            }
            if raw_text {
                out.push_str(&s); // Raw-text element content is emitted verbatim.
            } else {
                push_escaped_text(&s, out);
            }
        } else {
            serialize_node(child, out);
        }
    }

    out.push_str("</");
    out.push_str(tag);
    out.push('>');
}

fn push_escaped_text(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
}

fn push_escaped_attr(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\u{a0}' => out.push_str("&nbsp;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<!DOCTYPE html><html><head>
        <meta name="hfp-id" content="abc">
        <script id="hfp-schema" type="application/json">{"x":1}</script>
        <script id="hfp-data" type="application/json">{"a":1}</script>
        <script id="hfp-data-signature" type="application/pkcs7-signature">SIG</script>
        </head><body><p>Hello</p></body></html>"#;

    fn canon(s: &str) -> String {
        String::from_utf8(canonicalize(s.as_bytes()).unwrap()).unwrap()
    }

    #[test]
    fn empties_data_and_data_signature() {
        let out = canon(DOC);
        assert!(out.contains(r#"<script id="hfp-data" type="application/json"></script>"#));
        assert!(out.contains(
            r#"<script id="hfp-data-signature" type="application/pkcs7-signature"></script>"#
        ));
        // The schema block keeps its content.
        assert!(out.contains(r#"{"x":1}"#));
    }

    #[test]
    fn benign_mutations_share_a_hash() {
        let base = canonical_sha256_hex(DOC.as_bytes()).unwrap();

        // Different data -> same hash (data is emptied).
        let other_data = DOC.replace(r#"{"a":1}"#, r#"{"a":999,"b":"zzz"}"#);
        assert_eq!(canonical_sha256_hex(other_data.as_bytes()).unwrap(), base);

        // CRLF line endings.
        let crlf = DOC.replace('\n', "\r\n");
        assert_eq!(canonical_sha256_hex(crlf.as_bytes()).unwrap(), base);

        // Leading BOM.
        let bom = format!("\u{feff}{DOC}");
        assert_eq!(canonical_sha256_hex(bom.as_bytes()).unwrap(), base);

        // Reordered attributes on the meta tag.
        let reordered = DOC.replace(
            r#"<meta name="hfp-id" content="abc">"#,
            r#"<meta content="abc" name="hfp-id">"#,
        );
        assert_eq!(canonical_sha256_hex(reordered.as_bytes()).unwrap(), base);

        // An adjacent HTML comment.
        let commented = DOC.replace(
            r#"<script id="hfp-data" "#,
            r#"<!-- filled below --><script id="hfp-data" "#,
        );
        assert_eq!(canonical_sha256_hex(commented.as_bytes()).unwrap(), base);

        // Extra inter-tag whitespace (formatter-style).
        let spaced = DOC.replace("<head>", "<head>\n\n        ");
        assert_eq!(canonical_sha256_hex(spaced.as_bytes()).unwrap(), base);
    }

    #[test]
    fn duplicate_data_block_hard_fails() {
        let dup = DOC.replace(
            r#"<script id="hfp-schema" type="application/json">{"x":1}</script>"#,
            r#"<script id="hfp-data" type="application/json">{"dup":1}</script>"#,
        );
        assert_eq!(
            canonicalize(dup.as_bytes()),
            Err(Error::DuplicateBlock("hfp-data"))
        );
    }

    #[test]
    fn missing_data_signature_hard_fails() {
        let missing = DOC.replace(
            r#"<script id="hfp-data-signature" type="application/pkcs7-signature">SIG</script>"#,
            "",
        );
        assert_eq!(
            canonicalize(missing.as_bytes()),
            Err(Error::MissingBlock("hfp-data-signature"))
        );
    }

    #[test]
    fn invalid_utf8_hard_fails() {
        assert_eq!(canonicalize(&[0xff, 0xfe, 0x00]), Err(Error::InvalidUtf8));
    }
}
