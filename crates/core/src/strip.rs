//! Astro whitespace stripping with optional sourcemap support.
//!
//! This module rewrites whitespace-only gaps between CST nodes in Astro source. Instead of only
//! deleting whitespace bytes (which collapses columns), it rotates small delimiter/prefix tokens
//! across the gap to reduce column drift and keep edits predictable for mapping.
//!
//! Rationale:
//!
//! - In the ideal case, tools preserve sourcemaps end-to-end. In that case, whitespace could be
//!   deleted freely and upstream mappings would recover original locations.
//! - In practice, sourcemaps may be unavailable, dropped, or not threaded through every step.
//!   To reduce location drift in the stripped output, this module moves a small prefix/suffix
//!   token across a whitespace-only gap (“rotation”) instead of deleting bytes outright.
//!
//! Definitions:
//!
//! - A “gap” is the byte range between adjacent named children of container nodes (`document` and
//!   `element`) where the range is entirely whitespace.
//!
//! Transformations:
//!
//! 1. Rotate trailing delimiter right: if the previous node ends with a delimiter such as `>`,
//!    `/>`, `-->`, or `}`, move that delimiter to the end of the gap (immediately before the next
//!    node). Optionally move up to one indentation byte (two for `/>`) from the final line of the
//!    gap to before the gap to reduce column shifts.
//! 2. Rotate opener prefix left: if the previous node is `text` and the next node begins with an
//!    opener prefix such as `<!--`, `{`, or `<tag`/`</tag`, move that prefix so it becomes
//!    adjacent to the text and leave the whitespace after the prefix.
//!
//! Notes:
//!
//! - Whitespace inside an `html_interpolation` node (the `{ ... }` expression) is not rewritten,
//!   because it is part of JavaScript and can be semantically meaningful.
//! - All offsets in this module are byte offsets (tree-sitter’s model). Edits carry a per-byte
//!   origin map so sourcemaps can be created or rewritten.

use crate::{
    StripError,
    edit::{Edit, create_sourcemap, validate_edits},
    parse::parse_astro,
};

/// Configuration options for whitespace stripping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StripConfig {
    /// If true, preserves “section breaks” by skipping gaps that contain an empty line.
    ///
    /// A “blank line” here is defined as two consecutive line breaks within the whitespace gap
    /// (either `\n\n` or `\r\n\r\n`).
    pub preserve_blank_lines: bool,
}

/// Output code and its corresponding sourcemap JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAndSourcemap {
    /// The rewritten Astro source.
    pub code: String,
    /// The generated/re-written sourcemap JSON.
    pub sourcemap: String,
}

/// Strip inter-node whitespace and create a brand-new sourcemap.
///
/// This exists so callers can still obtain a sourcemap even if upstream tooling does not emit
/// one. The returned sourcemap maps the stripped output back to `source`.
pub fn strip_astro_whitespace(
    source: &str,
    source_filename: &str,
    config: &StripConfig,
) -> Result<CodeAndSourcemap, StripError> {
    let (code, edits) = rewrite(source, config)?;
    let sourcemap = create_sourcemap(source, &code, source_filename, &edits)?;
    Ok(CodeAndSourcemap { code, sourcemap })
}

/// Strip inter-node whitespace without producing a sourcemap.
///
/// This is the cheapest entry point if you don't need mappings.
pub fn strip_astro_whitespace_no_sourcemap(
    source: &str,
    config: &StripConfig,
) -> Result<String, StripError> {
    let (code, _) = rewrite(source, config)?;
    Ok(code)
}

/// Parse `source`, collect non-overlapping edits, apply them, and return `(output, edits)`.
///
/// This is the shared core used by all public entry points.
fn rewrite(source: &str, config: &StripConfig) -> Result<(String, Vec<Edit>), StripError> {
    let src = source.as_bytes();

    let tree = parse_astro(source)?;
    let root = tree.root_node();

    // Collect edits by walking the CST.
    let edits = collect_edits(source, root, config);

    // Validate edits for overlaps.
    validate_edits(src.len(), &edits)?;

    // Rebuild the output with byte-level origin mapping.
    let mut out = Vec::<u8>::with_capacity(src.len());
    let mut cursor = 0usize;
    for edit in &edits {
        // Copy unchanged bytes before the edit.
        if cursor < edit.start {
            out.extend_from_slice(&src[cursor..edit.start]);
        }

        // Apply the edit replacement.
        out.extend_from_slice(edit.replacement.as_bytes());
        cursor = edit.end;
    }

    // Copy any remaining unchanged bytes after the last edit.
    if cursor < src.len() {
        out.extend_from_slice(&src[cursor..]);
    }

    let out = String::from_utf8(out).expect("output must be utf-8");
    Ok((out, edits))
}

/// Walk the parsed AST and collect whitespace-gap rewrite edits.
///
/// Returns a sorted list of edits to apply to `source`.
fn collect_edits(source: &str, node: tree_sitter::Node<'_>, config: &StripConfig) -> Vec<Edit> {
    let mut edits: Vec<Edit> = Vec::new();

    // Iterative traversal that uses a TreeCursor and never indexes children by integer.
    //
    // This avoids:
    // - call stack overflows (recursive traversal)
    // - huge Vec growth on wide nodes (push-all-children approaches)
    // - wasm panics where `child_count`/`child(i)` can disagree for some nodes
    //
    // We only apply gap edits to “container” nodes (`document` and `element`). We intentionally
    // do NOT treat `html_interpolation` as a container because whitespace within `{ ... }` is
    // part of JavaScript and can be semantically meaningful.

    fn process_container_gaps(
        source: &str,
        node: tree_sitter::Node<'_>,
        config: &StripConfig,
        edits: &mut Vec<Edit>,
    ) {
        let mut cursor = node.walk();
        let mut prev_named: Option<tree_sitter::Node<'_>> = None;

        for next in node.named_children(&mut cursor) {
            let Some(prev) = prev_named else {
                prev_named = Some(next);
                continue;
            };
            prev_named = Some(next);

            if prev.end_byte() >= next.start_byte() {
                continue;
            }

            let gap_start = prev.end_byte();
            let gap_end = next.start_byte();

            let gap = &source[gap_start..gap_end];
            if gap.is_empty() || !gap.chars().all(|c| c.is_whitespace()) {
                continue;
            }

            // Preserve intentional section breaks.
            if config.preserve_blank_lines && contains_blank_line(gap) {
                continue;
            }

            // Case 1: rotate a trailing delimiter from the previous node (">", "/>", "-->", "}")
            // to sit immediately before the next node.
            if let Some(delim) = TrailingDelim::from_node(prev) {
                let delim_len = delim.len();
                if prev.end_byte() >= delim_len {
                    let delim_pos = prev.end_byte() - delim_len;
                    if source.as_bytes().get(delim_pos..prev.end_byte()) == Some(delim.bytes()) {
                        let (replacement, input_offset_for_output) =
                            rotate_delim_over_gap(delim, gap);

                        let output_byte_to_input_byte = input_offset_for_output
                            .iter()
                            .map(|&in_off| Some(delim_pos + in_off))
                            .collect();

                        edits.push(Edit {
                            start: delim_pos,
                            end: gap_end,
                            replacement,
                            output_byte_to_input_byte,
                            moved_delim_len: delim_len,
                        });
                        continue;
                    }
                }
            }

            // Case 2: if the previous node is text and the next node begins with an opener
            // ("<!--", "<tag", "</tag", "{"), rotate that opener left across the gap so it
            // becomes adjacent to the text.
            if prev.kind() == "text"
                && let Some(prefix_end) = opener_prefix_end(source, next)
                && prefix_end > gap_end
            {
                let prefix = &source.as_bytes()[gap_end..prefix_end];
                let (replacement, input_offset_for_output) = rotate_prefix_over_gap(prefix, gap);

                let output_byte_to_input_byte = input_offset_for_output
                    .iter()
                    .map(|&in_off| Some(gap_start + in_off))
                    .collect();

                edits.push(Edit {
                    start: gap_start,
                    end: prefix_end,
                    replacement,
                    output_byte_to_input_byte,
                    moved_delim_len: 0,
                });
            }
        }
    }

    let mut cursor = node.walk();
    'walk: loop {
        let current = cursor.node();
        let kind = current.kind();
        if kind == "document" || kind == "element" {
            process_container_gaps(source, current, config, &mut edits);
        }

        // Descend if possible.
        if cursor.goto_first_child() {
            continue;
        }

        // Otherwise, move to the next sibling, or walk up until we can.
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                break 'walk;
            }
        }
    }

    edits.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then(a.end.cmp(&b.end))
            .then(a.replacement.len().cmp(&b.replacement.len()))
    });

    edits
}

/// Trailing delimiters that can be rotated across a whitespace gap.
///
/// The delimiter is detected by node kind and then verified against source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrailingDelim {
    Gt,
    SlashGt,
    CommentEnd,
    RBrace,
}

impl TrailingDelim {
    /// Returns the literal bytes of the delimiter.
    fn bytes(self) -> &'static [u8] {
        match self {
            TrailingDelim::Gt => b">",
            TrailingDelim::SlashGt => b"/>",
            TrailingDelim::CommentEnd => b"-->",
            TrailingDelim::RBrace => b"}",
        }
    }

    /// Returns the delimiter length in bytes.
    fn len(self) -> usize {
        self.bytes().len()
    }

    /// Attempts to infer the delimiter type that ends `node`.
    ///
    /// For `element`, we inspect its last named child to find the real trailing token.
    fn from_node(node: tree_sitter::Node<'_>) -> Option<Self> {
        match node.kind() {
            "start_tag" | "end_tag" => Some(TrailingDelim::Gt),
            "self_closing_tag" => Some(TrailingDelim::SlashGt),
            "comment" => Some(TrailingDelim::CommentEnd),
            "html_interpolation" => Some(TrailingDelim::RBrace),
            "element" => {
                // Find the actual trailing delimiter from the element's last child.
                let mut cursor = node.walk();
                let last_child = node.named_children(&mut cursor).last()?;
                TrailingDelim::from_node(last_child)
            }
            _ => None,
        }
    }
}

/// Returns the byte offset of the end of the “opener prefix” for `next`.
///
/// This is used for the “rotate opener prefix left” transformation.
///
/// Examples (prefixes moved):
///
/// - `comment`: `<!--`
/// - `html_interpolation`: `{`
/// - `start_tag`/`end_tag`/`self_closing_tag`: `<tagname` (up through the `tag_name` node)
fn opener_prefix_end(source: &str, next: tree_sitter::Node<'_>) -> Option<usize> {
    let start = next.start_byte();
    let bytes = source.as_bytes();

    match next.kind() {
        "html_interpolation" => {
            if bytes.get(start) == Some(&b'{') {
                Some(start + 1)
            } else {
                None
            }
        }
        "comment" => {
            if bytes.get(start..start + 4) == Some(b"<!--") {
                Some(start + 4)
            } else {
                None
            }
        }
        "element" => {
            // Move "<" + tag_name for the element's start tag.
            let mut cursor = next.walk();
            let tag = next
                .named_children(&mut cursor)
                .find(|c| c.kind() == "start_tag" || c.kind() == "self_closing_tag")?;
            opener_prefix_end(source, tag)
        }
        "start_tag" | "end_tag" | "self_closing_tag" => {
            if bytes.get(start) != Some(&b'<') {
                return None;
            }
            let mut cursor = next.walk();
            let tag_name = next
                .named_children(&mut cursor)
                .find(|c| c.kind() == "tag_name")?;
            if tag_name.end_byte() > start {
                Some(tag_name.end_byte())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Returns true if a whitespace-only gap contains a “blank line”.
///
/// A blank line is defined as two consecutive line breaks (`\n\n` or `\r\n\r\n`).
fn contains_blank_line(ws: &str) -> bool {
    // Handles both LF and CRLF. We treat an empty line as any occurrence of
    // two consecutive line breaks in the whitespace gap.
    let b = ws.as_bytes();
    if b.windows(2).any(|w| w == b"\n\n") {
        return true;
    }
    if b.windows(4).any(|w| w == b"\r\n\r\n") {
        return true;
    }
    false
}

/// Rotates a trailing delimiter from `prev` across the whitespace `gap`.
///
/// Input segment: `delim + gap`
/// Output segment: `(optional stolen indent) + gap' + delim`
///
/// Returns `(replacement, input_offset_for_output)` where each output byte is mapped back to
/// its originating input offset within the input segment.
fn rotate_delim_over_gap(delim: TrailingDelim, gap: &str) -> (String, Vec<usize>) {
    // Original segment is: delim + gap
    // We want: (optional stolen indent) + gap' + delim
    // Where stolen indent is one space/tab "stolen" from the end of indentation on the line
    // where the next node begins, to preserve column numbers when possible.

    let delim_bytes = delim.bytes();
    let delim_len = delim_bytes.len();

    let gap_bytes = gap.as_bytes();
    let gap_len = gap_bytes.len();

    // Find the start of the final line within `gap`.
    let mut last_line_start = 0usize;
    for (i, &b) in gap_bytes.iter().enumerate() {
        if b == b'\n' {
            last_line_start = i + 1;
        }
    }

    // Identify "stealable" whitespace bytes on that final line.
    // For "/>" we steal up to 2 bytes to preserve the next node's column.
    // For other delimiters, keep the historical behavior (steal at most 1 byte).
    let max_steal = match delim {
        TrailingDelim::SlashGt => 2,
        _ => 1,
    };

    let mut steal_indices: Vec<usize> = Vec::new();
    if max_steal > 0 && last_line_start < gap_len {
        let mut i = gap_len;
        while i > last_line_start && steal_indices.len() < max_steal {
            let b = gap_bytes[i - 1];
            if matches!(b, b' ' | b'\t') {
                steal_indices.push(i - 1);
                i -= 1;
            } else {
                break;
            }
        }
        steal_indices.sort_unstable();
    }

    let mut replacement = Vec::<u8>::with_capacity(delim_len + gap_len);
    let mut input_offset_for_output: Vec<usize> = Vec::with_capacity(delim_len + gap_len);

    // Source segment indices:
    // - delim is at input offsets 0..delim_len
    // - gap bytes are at input offsets delim_len..(delim_len + gap_len)

    if !steal_indices.is_empty() {
        // Output stolen bytes first.
        for &steal_i in &steal_indices {
            replacement.push(gap_bytes[steal_i]);
            input_offset_for_output.push(delim_len + steal_i);
        }

        // Then output the gap minus the stolen bytes.
        for (i, &b) in gap_bytes.iter().enumerate() {
            if steal_indices.binary_search(&i).is_ok() {
                continue;
            }
            replacement.push(b);
            input_offset_for_output.push(delim_len + i);
        }

        // And finally the delimiter bytes.
        for (i, &b) in delim_bytes.iter().enumerate() {
            replacement.push(b);
            input_offset_for_output.push(i);
        }

        return (
            String::from_utf8(replacement).expect("replacement must be utf-8"),
            input_offset_for_output,
        );
    }

    // No indentation to steal; just rotate: gap + delim.
    for (i, &b) in gap_bytes.iter().enumerate() {
        replacement.push(b);
        input_offset_for_output.push(delim_len + i);
    }
    for (i, &b) in delim_bytes.iter().enumerate() {
        replacement.push(b);
        input_offset_for_output.push(i);
    }

    (
        String::from_utf8(replacement).expect("replacement must be utf-8"),
        input_offset_for_output,
    )
}

/// Rotates an opener prefix (e.g. `{`, `<!--`, `<tag`) left across a whitespace `gap`.
///
/// Input segment: `gap + prefix`
/// Output segment: `prefix + gap`
///
/// Returns `(replacement, input_offset_for_output)` where each output byte is mapped back to
/// its originating input offset within the input segment.
fn rotate_prefix_over_gap(prefix: &[u8], gap: &str) -> (String, Vec<usize>) {
    // Original segment is: gap + prefix
    // We want: prefix + gap
    let gap_bytes = gap.as_bytes();
    let gap_len = gap_bytes.len();
    let prefix_len = prefix.len();

    let mut replacement = Vec::<u8>::with_capacity(gap_len + prefix_len);
    let mut input_offset_for_output: Vec<usize> = Vec::with_capacity(gap_len + prefix_len);

    // Input segment indices:
    // - gap is at input offsets 0..gap_len
    // - prefix is at input offsets gap_len..(gap_len+prefix_len)

    // Output prefix first.
    for (i, &b) in prefix.iter().enumerate() {
        replacement.push(b);
        input_offset_for_output.push(gap_len + i);
    }

    // Then output the original gap.
    for (i, &b) in gap_bytes.iter().enumerate() {
        replacement.push(b);
        input_offset_for_output.push(i);
    }

    (
        String::from_utf8(replacement).expect("replacement must be utf-8"),
        input_offset_for_output,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strips whitespace using the default config.
    fn strip(src: &str) -> String {
        strip_astro_whitespace_no_sourcemap(src, &StripConfig::default()).unwrap()
    }

    /// Strips whitespace using a custom `preserve_blank_lines` setting.
    fn strip_cfg(src: &str, preserve_blank_lines: bool) -> String {
        let cfg = StripConfig {
            preserve_blank_lines,
        };
        strip_astro_whitespace_no_sourcemap(src, &cfg).unwrap()
    }

    /// Asserts that `map` is a permutation of `0..map.len()`.
    fn assert_is_permutation(map: &[usize]) {
        let mut v = map.to_vec();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v, (0..map.len()).collect::<Vec<_>>());
    }

    /// Rotates `>` over a newline-only gap.
    #[test]
    fn rotates_gt_over_newline_only() {
        let (out, map) = rotate_delim_over_gap(TrailingDelim::Gt, "\n");
        assert_eq!(out, "\n>");
        assert_eq!(map.len(), 2);
        assert_is_permutation(&map);
    }

    /// Rotates `>` over a newline+indent gap and steals one indent byte.
    #[test]
    fn rotates_gt_over_newline_and_indent_with_filler() {
        let (out, map) = rotate_delim_over_gap(TrailingDelim::Gt, "\n  ");
        assert_eq!(out, " \n >");
        assert_eq!(map.len(), 4);
        assert_is_permutation(&map);
    }

    /// Rotates `-->` over a newline+indent gap and steals one indent byte.
    #[test]
    fn rotates_comment_end_over_newline_and_indent_with_filler() {
        let (out, map) = rotate_delim_over_gap(TrailingDelim::CommentEnd, "\n  ");
        assert_eq!(out, " \n -->");
        assert_eq!(map.len(), 6);
        assert_is_permutation(&map);
    }

    /// Rotates `/>` over a newline+indent gap and steals up to two indent bytes.
    #[test]
    fn rotates_slash_gt_over_newline_and_indent_with_two_fillers() {
        let (out, map) = rotate_delim_over_gap(TrailingDelim::SlashGt, "\n  ");
        // Steal both indentation spaces so the next node keeps its original column.
        assert_eq!(out, "  \n/>");
        assert_eq!(map.len(), 5);
        assert_is_permutation(&map);
    }

    /// Creates a sourcemap when no input sourcemap is provided.
    #[test]
    fn emits_sourcemap_without_input() {
        let src = "<div>\n  <span>ok</span>\n</div>\n";
        let res = strip_astro_whitespace(src, "input.astro", &StripConfig::default()).unwrap();
        let sm = sourcemap::SourceMap::from_slice(res.sourcemap.as_bytes()).unwrap();
        assert_eq!(sm.get_source(0), Some("input.astro"));
    }

    /// Rotates an opener prefix (`{`) left over a gap.
    #[test]
    fn rotates_prefix_left_over_gap() {
        let (out, map) = rotate_prefix_over_gap(b"{", "\n  ");
        assert_eq!(out, "{\n  ");
        assert_eq!(map.len(), 4);
        assert_is_permutation(&map);
    }

    /// Detects blank lines in LF and CRLF whitespace.
    #[test]
    fn contains_blank_line_lf_and_crlf() {
        assert!(contains_blank_line("\n\n"));
        assert!(contains_blank_line(" \n\n  "));
        assert!(contains_blank_line("\r\n\r\n"));
        assert!(!contains_blank_line("\n  \n"));
    }

    /// End-tag delimiter rotation steals indentation when available.
    #[test]
    fn rewrite_rotates_gt_before_text_with_indent_steal() {
        let src = "<span>\n  text</span>";
        let out = strip(src);
        assert_eq!(out, "<span \n >text</span>");
    }

    /// End-tag delimiter rotation without indentation.
    #[test]
    fn rewrite_rotates_gt_before_text_without_indent() {
        let src = "<span>\ntext</span>";
        let out = strip(src);
        assert_eq!(out, "<span\n>text</span>");
    }

    /// Interpolation `}` delimiter rotation.
    #[test]
    fn rewrite_rotates_rbrace_before_next_node() {
        let src = "{a}\n  <b/>";
        let out = strip(src);
        assert_eq!(out, "{a \n }<b/>");
    }

    /// Comment end delimiter (`-->`) rotation does not corrupt comment bytes.
    #[test]
    fn rewrite_rotates_comment_end_before_next_node() {
        let src = "<!--c-->\n  <span/>";
        let out = strip(src);
        // Ensure we didn't corrupt the comment end delimiter.
        assert!(out.contains("--><span"));
        assert!(out.contains("-->"));
    }

    /// `<tagname` opener prefix rotation after text.
    #[test]
    fn rewrite_rotates_tag_prefix_left_after_text() {
        let src = "hi\n  <span/>";
        let out = strip(src);
        // Prefix is "<span" moved left; whitespace remains after it.
        assert_eq!(out, "hi<span\n  />");
    }

    /// `<!--` opener prefix rotation after text.
    #[test]
    fn rewrite_rotates_comment_prefix_left_after_text() {
        let src = "hi\n  <!--c-->";
        let out = strip(src);
        assert_eq!(out, "hi<!--\n  c-->");
    }

    /// `{` opener prefix rotation after text.
    #[test]
    fn rewrite_rotates_interpolation_prefix_left_after_text() {
        let src = "hi\n  {a}";
        let out = strip(src);
        assert_eq!(out, "hi{\n  a}");
    }

    /// By default, blank-line gaps are not preserved.
    #[test]
    fn rewrite_does_not_preserve_blank_line_gaps_by_default() {
        let src = "<a></a>\n\n<b/>";
        let out = strip(src);
        assert_ne!(out, src);

        let src_crlf = "<a></a>\r\n\r\n<b/>";
        let out_crlf = strip(src_crlf);
        assert_ne!(out_crlf, src_crlf);
    }

    /// With config enabled, blank-line gaps are preserved.
    #[test]
    fn rewrite_preserves_blank_line_gaps_with_config() {
        let src = "<a></a>\n\n<b/>";
        let out = strip_cfg(src, true);
        assert_eq!(out, src);

        let src_crlf = "<a></a>\r\n\r\n<b/>";
        let out_crlf = strip_cfg(src_crlf, true);
        assert_eq!(out_crlf, src_crlf);
    }

    /// Whitespace inside `{ ... }` interpolation expressions is preserved.
    #[test]
    fn rewrite_does_not_touch_whitespace_inside_interpolation_expression() {
        let src = "{ a +  b }";
        let out = strip(src);
        assert_eq!(out, src);
    }

    /// Delimiter rotation when an end tag is followed by a start tag.
    #[test]
    fn end_tag_then_start_tag() {
        let src = "<a>x</a>\n  <b></b>";
        let out = strip(src);
        assert_eq!(out, "<a>x</a \n ><b></b>");
    }

    /// Delimiter rotation when an end tag is followed by a self-closing tag.
    #[test]
    fn end_tag_then_self_closing_tag() {
        let src = "<a>x</a>\n  <b/>";
        let out = strip(src);
        assert_eq!(out, "<a>x</a \n ><b/>");
    }

    /// Delimiter rotation when an end tag is followed by interpolation.
    #[test]
    fn end_tag_then_start_interpolation() {
        let src = "<a>x</a>\n  {b}";
        let out = strip(src);
        assert_eq!(out, "<a>x</a \n >{b}");
    }

    /// Delimiter rotation when an end tag is followed by text.
    #[test]
    fn end_tag_then_text() {
        let src = "<a>x</a>\n  text";
        let out = strip(src);
        assert_eq!(out, "<a>x</a \n >text");
    }

    /// `/>` delimiter rotation when followed by a start tag.
    #[test]
    fn self_closing_tag_then_start_tag() {
        let src = "<a/>\n  <b>y</b>";
        let out = strip(src);
        assert_eq!(out, "<a  \n/><b>y</b>");
    }

    /// `/>` delimiter rotation when followed by another self-closing tag.
    #[test]
    fn self_closing_tag_then_self_closing_tag() {
        let src = "<a/>\n  <b/>";
        let out = strip(src);
        assert_eq!(out, "<a  \n/><b/>");
    }

    /// `/>` delimiter rotation when followed by interpolation.
    #[test]
    fn self_closing_tag_then_start_interpolation() {
        let src = "<a/>\n  {b}";
        let out = strip(src);
        assert_eq!(out, "<a  \n/>{b}");
    }

    /// `/>` delimiter rotation when followed by text.
    #[test]
    fn self_closing_tag_then_text() {
        let src = "<a/>\n  text";
        let out = strip(src);
        assert_eq!(out, "<a  \n/>text");
    }

    /// `}` delimiter rotation when interpolation is followed by a start tag.
    #[test]
    fn end_interpolation_then_start_tag() {
        let src = "{a}\n  <b>y</b>";
        let out = strip(src);
        assert_eq!(out, "{a \n }<b>y</b>");
    }

    /// `}` delimiter rotation when interpolation is followed by a self-closing tag.
    #[test]
    fn end_interpolation_then_self_closing_tag() {
        let src = "{a}\n  <b/>";
        let out = strip(src);
        assert_eq!(out, "{a \n }<b/>");
    }

    /// `}` delimiter rotation when interpolation is followed by interpolation.
    #[test]
    fn end_interpolation_then_start_interpolation() {
        let src = "{a}\n  {b}";
        let out = strip(src);
        assert_eq!(out, "{a \n }{b}");
    }

    /// `}` delimiter rotation when interpolation is followed by text.
    #[test]
    fn end_interpolation_then_text() {
        let src = "{a}\n  text";
        let out = strip(src);
        assert_eq!(out, "{a \n }text");
    }

    /// Opener prefix rotation when text is followed by a start tag.
    #[test]
    fn text_then_start_tag() {
        let src = "hi\n  <b>y</b>";
        let out = strip(src);
        assert_eq!(out, "hi<b\n  >y</b>");
    }

    /// Opener prefix rotation when text is followed by a self-closing tag.
    #[test]
    fn text_then_self_closing_tag() {
        let src = "hi\n  <b/>";
        let out = strip(src);
        assert_eq!(out, "hi<b\n  />");
    }

    /// Whitespace between adjacent text nodes is never modified.
    #[test]
    fn text_then_text_do_not_strip() {
        // Even if tree-sitter ever produces adjacent text nodes, we must not
        // strip whitespace between them. This is a simple no-op sanity check.
        let src = "hi\n  there";
        let out = strip(src);
        assert_eq!(out, src);
    }
}
