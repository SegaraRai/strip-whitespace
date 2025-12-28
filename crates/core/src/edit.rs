//! Edit and sourcemap utilities.
//!
//! This module defines [`Edit`], a non-overlapping byte-range replacement over the original
//! Astro source, plus helpers to produce sourcemaps that stay useful after whitespace stripping.
//!
//! Key ideas:
//!
//! - Internal offsets are byte-based (tree-sitter’s model).
//! - Sourcemap columns are emitted/consumed as UTF-16 code unit columns (matching typical JS
//!   sourcemap consumers).
//! - Each edit can carry a per-output-byte origin map (see [`Edit::output_byte_to_input_byte`])
//!   so moved/rotated bytes map back to their original positions.
//! - Sourcemaps can either be created from scratch for the stripped output or rewritten from
//!   an existing input sourcemap.
//!
//! Invariants:
//!
//! - `edits` must be sorted by ascending `start` and must not overlap (validated upstream).
//! - Replacement strings are treated as raw bytes; mappings are best-effort if lengths mismatch.

use std::cmp::Ordering;

use crate::{StripError, utf16::Utf16Index};

/// A source-to-source rewrite applied to the generated Astro code.
///
/// This crate represents changes as edits over byte ranges in the input (pre-strip) code.
/// Edits are required to be non-overlapping (enforced upstream in `strip.rs`) and are applied
/// left-to-right for mapping math and right-to-left for actual text replacement.
#[derive(Debug, Clone)]
pub struct Edit {
    /// Start byte offset (inclusive) in the input code.
    pub start: usize,
    /// End byte offset (exclusive) in the input code.
    pub end: usize,
    /// Replacement text inserted into the output code.
    pub replacement: String,
    /// For each output byte in `replacement`, the originating byte offset in the input code.
    ///
    /// - `Some(input_byte)` means this output byte came from `input_code.as_bytes()[input_byte]`.
    /// - `None` means this output byte is newly inserted and should be considered unmapped.
    ///
    /// This crate uses byte offsets internally. Sourcemap columns are UTF-16 code unit columns.
    pub output_byte_to_input_byte: Vec<Option<usize>>,
    /// If non-zero, the last `moved_delim_len` bytes of `replacement` are a delimiter moved from
    /// elsewhere within the edit span (e.g. `>`, `/>`, `-->`, `}`). We add extra sourcemap anchors
    /// around this suffix to prevent its mapping span from "bleeding" into the next token.
    pub moved_delim_len: usize,
}

/// Create a brand-new sourcemap for `output_code`, mapping back to `input_code`.
///
/// Mapping model:
///
/// - Unchanged bytes map 1:1.
/// - Bytes originating from moved/rotated content map to their original input bytes via
///   [`Edit::output_byte_to_input_byte`].
/// - Inserted bytes are marked unmapped (`u32::MAX`/no source) via `None` origins.
///
/// Notes/limitations:
///
/// - Line/column are UTF-16 code unit based (JS sourcemap convention).
/// - If `output_code` length doesn't match what the edits imply, we emit a best-effort map for
///   the overlapping prefix.
/// - `edits` must be non-overlapping and in ascending `start` order (enforced upstream).
pub fn create_sourcemap(
    input_code: &str,
    output_code: &str,
    source_filename: &str,
    edits: &[Edit],
) -> Result<String, StripError> {
    // Best-effort behavior: if the provided `output_code` doesn't match the edit math for
    // some reason, we'll still emit a map for the overlapping prefix.
    let out_len = output_code.len();
    let in_len = input_code.len();

    let in_line_starts = compute_line_starts(input_code);
    let out_line_starts = compute_line_starts(output_code);
    let in_utf16 = Utf16Index::new(input_code, &in_line_starts);
    let out_utf16 = Utf16Index::new(output_code, &out_line_starts);

    let (edit_out_spans, expected_out_len) = compute_output_spans(in_len, edits);
    let map_len = out_len.min(expected_out_len);
    let out_to_in = build_output_to_input_map(in_len, map_len, &edit_out_spans);

    let mut builder = sourcemap::SourceMapBuilder::new(None);
    builder.add_source(source_filename);
    builder.set_source_contents(0, Some(input_code));

    // Anchor each output line start.
    for (out_line, &out_start) in out_line_starts.iter().enumerate() {
        if out_start >= map_len {
            break;
        }

        match out_to_in[out_start] {
            Some(in_byte) if in_byte < in_len => {
                let (in_line, in_col) = in_utf16.byte_to_line_utf16_col(in_byte);
                builder.add(
                    out_line as u32,
                    0,
                    in_line as u32,
                    in_col as u32,
                    Some(source_filename),
                    None,
                    false,
                );
            }
            _ => {
                // Unmapped: inserted bytes, or mismatch beyond best-effort prefix.
                builder.add(out_line as u32, 0, u32::MAX, u32::MAX, None, None, false);
            }
        }
    }

    // Add extra anchors around moved delimiters so column mapping stays useful.
    for (e, out_start, out_end) in edit_out_spans {
        // Start of the edit segment.
        if out_start < map_len {
            add_anchor_create(
                &mut builder,
                &out_utf16,
                &in_utf16,
                &out_to_in,
                out_start,
                source_filename,
            );
        }

        // Beginning of the delimiter in the output (it ends up at the end of the segment).
        if e.moved_delim_len > 0 && out_end >= e.moved_delim_len {
            let out_delim = out_end - e.moved_delim_len;
            if out_delim < map_len {
                add_anchor_create(
                    &mut builder,
                    &out_utf16,
                    &in_utf16,
                    &out_to_in,
                    out_delim,
                    source_filename,
                );
            }

            // For multi-byte delimiters (e.g. "-->"), also anchor the last byte so the
            // trailing '>' doesn't inherit the mapping of the first '-'.
            if e.moved_delim_len > 1 {
                let out_delim_last = out_end.saturating_sub(1);
                if out_delim_last < map_len {
                    add_anchor_create(
                        &mut builder,
                        &out_utf16,
                        &in_utf16,
                        &out_to_in,
                        out_delim_last,
                        source_filename,
                    );
                }
            }
        }

        // Boundary at the start of the next node (right after a moved delimiter).
        // This prevents the moved delimiter's mapping span from covering the next token.
        if e.moved_delim_len > 0 && out_end < map_len {
            add_anchor_create(
                &mut builder,
                &out_utf16,
                &in_utf16,
                &out_to_in,
                out_end,
                source_filename,
            );
        }
    }

    let out_map = builder.into_sourcemap();
    let mut buf: Vec<u8> = Vec::new();
    out_map.to_writer(&mut buf)?;
    Ok(String::from_utf8(buf).expect("sourcemap JSON must be utf-8"))
}

/// Rewrite an existing sourcemap for `input_code` so it applies to `output_code`.
///
/// See [`create_sourcemap`] for mapping model notes, limitations, and requirements.
///
/// This is used when the caller already has a sourcemap describing how `input_code` maps back to
/// original sources (e.g. from a prior compilation step). We remap the generated positions
/// through the whitespace-stripping edits and emit a new sourcemap for `output_code`.
///
/// Behavior:
///
/// - Tokens that land in deleted regions are dropped (no destination position exists).
/// - Inserted bytes are emitted as unmapped (`u32::MAX`/no source).
/// - Input/output columns are treated as UTF-16 code unit columns.
pub fn rewrite_sourcemap(
    input_code: &str,
    output_code: &str,
    input_sourcemap_json: &str,
    edits: &[Edit],
) -> Result<String, StripError> {
    let input_map = sourcemap::SourceMap::from_slice(input_sourcemap_json.as_bytes())?;

    let in_line_starts = compute_line_starts(input_code);
    let out_line_starts = compute_line_starts(output_code);
    let in_utf16 = Utf16Index::new(input_code, &in_line_starts);
    let out_utf16 = Utf16Index::new(output_code, &out_line_starts);

    let in_len = input_code.len();
    let out_len = output_code.len();

    let (edit_out_spans, expected_out_len) = compute_output_spans(in_len, edits);
    let map_out_len = out_len.min(expected_out_len);
    let out_to_in = build_output_to_input_map(in_len, map_out_len, &edit_out_spans);
    let in_to_out = build_input_to_output_map(in_len, map_out_len, &edit_out_spans);

    let mut mappings: Vec<Mapping<'_>> = Vec::new();

    for token in input_map.tokens() {
        let in_line = token.get_dst_line() as usize;
        let in_col = token.get_dst_col() as usize;

        let Some(in_byte) = in_utf16.line_utf16_col_to_byte(in_line, in_col) else {
            continue;
        };
        if in_byte >= in_to_out.len() {
            continue;
        }

        let Some(out_byte) = in_to_out[in_byte] else {
            // This generated byte was deleted or otherwise removed in the output.
            continue;
        };
        let (out_line, out_col) = out_utf16.byte_to_line_utf16_col(out_byte);

        mappings.push(Mapping {
            dst_line: out_line as u32,
            dst_col: out_col as u32,
            src_line: token.get_src_line(),
            src_col: token.get_src_col(),
            source: token.get_source(),
            name: token.get_name(),
        });
    }

    // Add boundary mappings at the start of the next node after a moved delimiter.
    for (e, out_start, out_end) in edit_out_spans {
        if e.moved_delim_len == 0 {
            continue;
        }

        // 1) Start of the edit in output.
        if out_start < map_out_len {
            add_anchor_rewrite(
                &input_map,
                &mut mappings,
                &in_utf16,
                &out_utf16,
                &out_to_in,
                out_start,
            );
        }

        // 2) Start of the moved delimiter suffix in output.
        if e.moved_delim_len > 0 && out_end >= e.moved_delim_len {
            let out_delim = out_end - e.moved_delim_len;
            if out_delim < map_out_len {
                add_anchor_rewrite(
                    &input_map,
                    &mut mappings,
                    &in_utf16,
                    &out_utf16,
                    &out_to_in,
                    out_delim,
                );
            }

            // 3) Last byte of the moved delimiter suffix.
            if e.moved_delim_len > 1 {
                let out_delim_last = out_end.saturating_sub(1);
                if out_delim_last < map_out_len {
                    add_anchor_rewrite(
                        &input_map,
                        &mut mappings,
                        &in_utf16,
                        &out_utf16,
                        &out_to_in,
                        out_delim_last,
                    );
                }
            }
        }

        // 4) Boundary at the start of the next node (right after a moved delimiter).
        // This prevents the moved delimiter's mapping span from covering the next token.
        if out_end < map_out_len && e.end < in_len {
            let (out_line, out_col) = out_utf16.byte_to_line_utf16_col(out_end);
            let (in_line, in_col) = in_utf16.byte_to_line_utf16_col(e.end);

            let tok = input_map.lookup_token(in_line as u32, in_col as u32);
            let (src_line, src_col, source, name) = match tok {
                Some(t) => (
                    t.get_src_line(),
                    t.get_src_col(),
                    t.get_source(),
                    t.get_name(),
                ),
                None => (u32::MAX, u32::MAX, None, None),
            };

            mappings.push(Mapping {
                dst_line: out_line as u32,
                dst_col: out_col as u32,
                src_line,
                src_col,
                source,
                name,
            });
        }
    }

    sort_and_dedup_mappings(&mut mappings);

    let mut builder = sourcemap::SourceMapBuilder::new(None);
    for m in mappings {
        // Force point mappings; spans are controlled by adjacent entries.
        builder.add(
            m.dst_line, m.dst_col, m.src_line, m.src_col, m.source, m.name, false,
        );
    }

    let out_map = builder.into_sourcemap();
    let mut buf: Vec<u8> = Vec::new();
    out_map.to_writer(&mut buf)?;
    Ok(String::from_utf8(buf).expect("sourcemap JSON must be utf-8"))
}

/// Validate edit invariants required by this module.
///
/// This performs cheap structural checks only:
///
/// - `start <= end` and `end <= input_len`
/// - edits are non-overlapping (when provided in ascending `start` order)
/// - `output_byte_to_input_byte.len() == replacement.len()`
/// - `moved_delim_len <= replacement.len()`
/// - any `Some(input_byte)` origin is `< input_len`
///
/// This does not verify that `replacement` bytes actually match the referenced input bytes;
/// callers are responsible for constructing correct edits.
pub fn validate_edits(input_len: usize, edits: &[Edit]) -> Result<(), StripError> {
    let mut prev_end: usize = 0;
    for (idx, e) in edits.iter().enumerate() {
        if e.start > e.end {
            return Err(StripError::InvalidEdit(format!(
                "start > end at index {idx}: start={}, end={}",
                e.start, e.end
            )));
        }
        if e.end > input_len {
            return Err(StripError::InvalidEdit(format!(
                "edit out of bounds at index {idx}: end={} > input_len={}",
                e.end, input_len
            )));
        }
        if idx > 0 && e.start < prev_end {
            return Err(StripError::OverlappingEdits {
                a_start: edits[idx - 1].start,
                a_end: edits[idx - 1].end,
                b_start: e.start,
                b_end: e.end,
            });
        }
        if e.output_byte_to_input_byte.len() != e.replacement.len() {
            return Err(StripError::InvalidEdit(format!(
                "output_byte_to_input_byte length mismatch at index {idx}: map_len={}, replacement_len={}",
                e.output_byte_to_input_byte.len(),
                e.replacement.len()
            )));
        }
        if e.moved_delim_len > e.replacement.len() {
            return Err(StripError::InvalidEdit(format!(
                "moved_delim_len too large at index {idx}: moved_delim_len={} > replacement_len={}",
                e.moved_delim_len,
                e.replacement.len()
            )));
        }
        for (out_off, maybe_in_byte) in e.output_byte_to_input_byte.iter().copied().enumerate() {
            match maybe_in_byte {
                Some(in_byte) if in_byte >= input_len => {
                    return Err(StripError::InvalidEdit(format!(
                        "mapped input byte out of bounds at index {idx}: out_off={out_off}, in_byte={} >= input_len={}",
                        in_byte, input_len
                    )));
                }
                _ => {}
            }
        }

        prev_end = e.end;
    }
    Ok(())
}

/// A single point mapping entry (destination -> original source).
///
/// `dst_*` refers to the post-strip generated code (`output_code`). `src_*` refers to the
/// original sources referenced by the input sourcemap.
#[derive(Debug, Clone, Copy)]
struct Mapping<'a> {
    /// Destination line in the post-strip generated code.
    /// Ranging from `0..=u32::MAX-1`; `u32::MAX` means unmapped.
    dst_line: u32,
    /// Destination column in the post-strip generated code.
    /// Ranging from `0..=u32::MAX-1`; `u32::MAX` means unmapped.
    dst_col: u32,
    /// Original source line.
    /// Ranging from `0..=u32::MAX-1`; `u32::MAX` means unmapped.
    src_line: u32,
    /// Original source column.
    /// Ranging from `0..=u32::MAX-1`; `u32::MAX` means unmapped.
    src_col: u32,
    /// Original source filename (if any).
    source: Option<&'a str>,
    /// Original symbol name (if any).
    name: Option<&'a str>,
}

/// Sort mappings deterministically and remove duplicates.
///
/// Ordering is by destination position (line/col). When multiple entries target the same
/// destination position, mapped entries (i.e. not `u32::MAX`) are preferred over unmapped.
fn sort_and_dedup_mappings(mappings: &mut Vec<Mapping<'_>>) {
    // Deterministic ordering: by dst position, and for the same dst position,
    // ensure mapped entries sort before unmapped ones.
    mappings.sort_by(
        |a, b| match (a.dst_line, a.dst_col).cmp(&(b.dst_line, b.dst_col)) {
            Ordering::Equal => {
                let a_mapped = a.src_line != u32::MAX;
                let b_mapped = b.src_line != u32::MAX;
                match (a_mapped, b_mapped) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => Ordering::Equal,
                }
            }
            other => other,
        },
    );
    mappings.dedup_by(|a, b| a.dst_line == b.dst_line && a.dst_col == b.dst_col);
}

/// Compute the byte offsets where each line starts.
///
/// The returned vector always includes `0` (first line) and then `i+1` for every `\n` byte at
/// index `i`.
fn compute_line_starts(s: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in s.as_bytes().iter().copied().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Convert a (line, column) pair into an absolute byte offset.
///
/// Columns are treated as byte columns relative to the line start.
#[cfg(test)]
fn line_col_to_byte(line_starts: &[usize], line: usize, col: usize) -> Option<usize> {
    let start = line_starts.get(line).copied()?;
    Some(start + col)
}

/// Convert an absolute byte offset into a (line, column) pair.
///
/// If `byte` is past the end of the string, the returned column will extend past the final line
/// start (this is intentional; used for best-effort behavior).
#[cfg(test)]
fn byte_to_line_col(line_starts: &[usize], byte: usize) -> (usize, usize) {
    // Find the last line start <= byte.
    let line = match line_starts.binary_search_by(|&probe| {
        if probe <= byte {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }) {
        Ok(i) => i,
        Err(0) => 0,
        Err(i) => i - 1,
    };
    let col = byte.saturating_sub(line_starts[line]);
    (line, col)
}

/// Compute each edit's span in the output code.
///
/// Returns a list of tuples `(edit, out_start, out_end)` and the expected output length.
/// Output spans are derived by tracking the cumulative length delta introduced by edits.
fn compute_output_spans(input_len: usize, edits: &[Edit]) -> (Vec<(&Edit, usize, usize)>, usize) {
    let mut spans: Vec<(&Edit, usize, usize)> = Vec::with_capacity(edits.len());
    let mut delta: isize = 0;
    for e in edits {
        let out_start = (e.start as isize + delta) as usize;
        let out_end = out_start + e.replacement.len();
        spans.push((e, out_start, out_end));
        delta += e.replacement.len() as isize - (e.end - e.start) as isize;
    }

    let expected_out_len = (input_len as isize + delta).max(0) as usize;
    (spans, expected_out_len)
}

/// Build a map from output byte offset to input byte offset.
///
/// The returned vector has length `output_len`. `Some(input_byte)` means the output byte came
/// from the input at that byte offset; `None` means the output byte is inserted/unmapped.
fn build_output_to_input_map(
    input_len: usize,
    output_len: usize,
    edit_out_spans: &[(&Edit, usize, usize)],
) -> Vec<Option<usize>> {
    let mut out_to_in: Vec<Option<usize>> = vec![None; output_len];

    let mut in_cursor: usize = 0;
    let mut out_cursor: usize = 0;

    for &(e, out_start, out_end) in edit_out_spans {
        // Unchanged region before the edit.
        if e.start > in_cursor {
            let len = e.start - in_cursor;
            for i in 0..len {
                let in_byte = in_cursor + i;
                let out_byte = out_cursor + i;
                if out_byte >= output_len {
                    return out_to_in;
                }
                if in_byte < input_len {
                    out_to_in[out_byte] = Some(in_byte);
                }
            }
            out_cursor += len;
        }

        // Edit replacement bytes.
        if out_start != out_cursor {
            // If edits are inconsistent or output length differs, best-effort: trust cursor.
            out_cursor = out_start;
        }

        for (j, maybe_in_byte) in e.output_byte_to_input_byte.iter().copied().enumerate() {
            let out_byte = out_cursor + j;
            if out_byte >= output_len {
                return out_to_in;
            }
            if let Some(in_byte) = maybe_in_byte.filter(|&b| b < input_len) {
                out_to_in[out_byte] = Some(in_byte);
            }
        }

        in_cursor = e.end;
        out_cursor = out_end;
    }

    // Trailing unchanged region.
    if in_cursor < input_len {
        for in_byte in in_cursor..input_len {
            let out_byte = out_cursor + (in_byte - in_cursor);
            if out_byte >= output_len {
                break;
            }
            out_to_in[out_byte] = Some(in_byte);
        }
    }

    out_to_in
}

/// Build a map from input byte offset to output byte offset.
///
/// The returned vector has length `input_len`. `Some(out_byte)` means the input byte survives in
/// the output at that position; `None` means it was deleted or otherwise not present.
fn build_input_to_output_map(
    input_len: usize,
    output_len: usize,
    edit_out_spans: &[(&Edit, usize, usize)],
) -> Vec<Option<usize>> {
    let mut in_to_out: Vec<Option<usize>> = vec![None; input_len];

    let mut in_cursor: usize = 0;
    let mut out_cursor: usize = 0;

    for &(e, out_start, out_end) in edit_out_spans {
        // Unchanged region before the edit.
        if e.start > in_cursor {
            let len = e.start - in_cursor;
            for i in 0..len {
                let in_byte = in_cursor + i;
                let out_byte = out_cursor + i;
                if out_byte >= output_len {
                    return in_to_out;
                }
                in_to_out[in_byte] = Some(out_byte);
            }
            out_cursor += len;
        }

        // Edit bytes: map origins back to their output positions.
        if out_start != out_cursor {
            out_cursor = out_start;
        }
        for (j, maybe_in_byte) in e.output_byte_to_input_byte.iter().copied().enumerate() {
            let out_byte = out_cursor + j;
            if out_byte >= output_len {
                break;
            }
            if let Some(in_byte) = maybe_in_byte.filter(|&b| b < input_len) {
                in_to_out[in_byte] = Some(out_byte);
            }
        }

        in_cursor = e.end;
        out_cursor = out_end;
    }

    // Trailing unchanged region.
    if in_cursor < input_len {
        for (i, slot) in in_to_out[in_cursor..].iter_mut().enumerate() {
            let out_byte = out_cursor + i;
            if out_byte >= output_len {
                break;
            }
            *slot = Some(out_byte);
        }
    }

    in_to_out
}

/// Add a single mapping entry for [`create_sourcemap`].
///
/// If `out_byte` is unmapped, this emits an explicit unmapped token entry.
fn add_anchor_create(
    builder: &mut sourcemap::SourceMapBuilder,
    out_utf16: &Utf16Index<'_>,
    in_utf16: &Utf16Index<'_>,
    out_to_in: &[Option<usize>],
    out_byte: usize,
    source_filename: &str,
) {
    let (out_line, out_col) = out_utf16.byte_to_line_utf16_col(out_byte);
    match out_to_in.get(out_byte).copied().flatten() {
        Some(in_byte) => {
            let (in_line, in_col) = in_utf16.byte_to_line_utf16_col(in_byte);
            builder.add(
                out_line as u32,
                out_col as u32,
                in_line as u32,
                in_col as u32,
                Some(source_filename),
                None,
                false,
            );
        }
        None => {
            builder.add(
                out_line as u32,
                out_col as u32,
                u32::MAX,
                u32::MAX,
                None,
                None,
                false,
            );
        }
    }
}

/// Add a single mapping entry for [`rewrite_sourcemap`].
///
/// We translate `out_byte` -> (input generated position) using `out_to_in`, then look up the
/// corresponding token in the input sourcemap. If `out_byte` is unmapped, we emit an unmapped
/// entry.
fn add_anchor_rewrite<'a>(
    input_map: &'a sourcemap::SourceMap,
    mappings: &mut Vec<Mapping<'a>>,
    in_utf16: &Utf16Index<'_>,
    out_utf16: &Utf16Index<'_>,
    out_to_in: &[Option<usize>],
    out_byte: usize,
) {
    let Some(in_byte) = out_to_in.get(out_byte).copied().flatten() else {
        let (out_line, out_col) = out_utf16.byte_to_line_utf16_col(out_byte);
        mappings.push(Mapping {
            dst_line: out_line as u32,
            dst_col: out_col as u32,
            src_line: u32::MAX,
            src_col: u32::MAX,
            source: None,
            name: None,
        });
        return;
    };

    let (out_line, out_col) = out_utf16.byte_to_line_utf16_col(out_byte);
    let (in_line, in_col) = in_utf16.byte_to_line_utf16_col(in_byte);

    let tok = input_map.lookup_token(in_line as u32, in_col as u32);
    let (src_line, src_col, source, name) = match tok {
        Some(t) => (
            t.get_src_line(),
            t.get_src_col(),
            t.get_source(),
            t.get_name(),
        ),
        None => (u32::MAX, u32::MAX, None, None),
    };

    mappings.push(Mapping {
        dst_line: out_line as u32,
        dst_col: out_col as u32,
        src_line,
        src_col,
        source,
        name,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Apply a single edit to `input` and return the resulting output.
    fn apply_single_edit(input: &str, edit: &Edit) -> String {
        let mut out = input.to_string();
        out.replace_range(edit.start..edit.end, &edit.replacement);
        out
    }

    /// Regression test: moved delimiter mapping must not bleed into the next token.
    #[test]
    fn create_sourcemap_separates_moved_gt_and_next_lt() {
        // Input:  <a>\n<b>
        // Output: <a\n><b>  (the '>' moved to just before the '<b>')
        let input = "<a>\n<b>";
        let edit = Edit {
            start: 2,
            end: 4,
            replacement: "\n>".to_string(),
            // segment is ">\n" at input bytes 2..4 -> output is "\n>"
            output_byte_to_input_byte: vec![Some(3), Some(2)],
            moved_delim_len: 1,
        };
        let output = apply_single_edit(input, &edit);
        assert_eq!(output, "<a\n><b>");

        let sm_json = create_sourcemap(input, &output, "input.astro", &[edit]).unwrap();
        let sm = sourcemap::SourceMap::from_slice(sm_json.as_bytes()).unwrap();

        // In output, the moved '>' is at line 1 col 0; it should map to the original '>' at line 0 col 2.
        let t_gt = sm.lookup_token(1, 0).expect("token for moved '>'");
        assert_eq!(t_gt.get_source(), Some("input.astro"));
        assert_eq!(t_gt.get_src_line(), 0);
        assert_eq!(t_gt.get_src_col(), 2);

        // In output, the '<' is at line 1 col 1; it should map to the original '<' at line 1 col 0.
        // This is the regression: without a boundary mapping, lookup_token(1,1) could resolve to the '>' token.
        let t_lt = sm.lookup_token(1, 1).expect("token for '<'");
        assert_eq!(t_lt.get_source(), Some("input.astro"));
        assert_eq!(t_lt.get_src_line(), 1);
        assert_eq!(t_lt.get_src_col(), 0);
    }

    /// Regression test: multi-byte delimiters ("-->") require extra anchors.
    #[test]
    fn create_sourcemap_separates_moved_comment_end_and_next_lt() {
        // Rotate "-->" across "\n" so the comment end is adjacent to the next tag.
        let input = "<!--c-->\n<span>";
        let start = input.find("-->").unwrap();
        let end = start + 4; // "-->\n"
        let edit = Edit {
            start,
            end,
            replacement: "\n-->".to_string(),
            // input segment is ["-","-",">","\n"] -> output is ["\n","-","-",">"]
            output_byte_to_input_byte: vec![
                Some(start + 3),
                Some(start),
                Some(start + 1),
                Some(start + 2),
            ],
            moved_delim_len: 3,
        };
        let output = apply_single_edit(input, &edit);
        assert!(output.contains("--><span>"));

        let sm_json = create_sourcemap(input, &output, "input.astro", &[edit]).unwrap();
        let sm = sourcemap::SourceMap::from_slice(sm_json.as_bytes()).unwrap();

        // Locate the "--><" boundary in output.
        let boundary = output.find("--><").unwrap();
        let out_gt = boundary + 2;
        let out_lt = boundary + 3;

        let out_starts = compute_line_starts(&output);
        let out_utf16 = Utf16Index::new(&output, &out_starts);
        let (gt_line, gt_col) = out_utf16.byte_to_line_utf16_col(out_gt);
        let (lt_line, lt_col) = out_utf16.byte_to_line_utf16_col(out_lt);

        let t_gt = sm
            .lookup_token(gt_line as u32, gt_col as u32)
            .expect("token for '>'");
        assert_eq!(t_gt.get_source(), Some("input.astro"));
        // Original '>' in "<!--c-->" is on line 0, last column.
        assert_eq!(t_gt.get_src_line(), 0);
        assert_eq!(t_gt.get_src_col(), 7);

        let t_lt = sm
            .lookup_token(lt_line as u32, lt_col as u32)
            .expect("token for '<'");
        assert_eq!(t_lt.get_source(), Some("input.astro"));
        // Original '<' of <span> is at line 1 col 0.
        assert_eq!(t_lt.get_src_line(), 1);
        assert_eq!(t_lt.get_src_col(), 0);
    }

    /// Ensure `rewrite_sourcemap` preserves distinct mappings after delimiter movement.
    #[test]
    fn rewrite_sourcemap_separates_moved_gt_and_next_lt() {
        let input = "<a>\n<b>";
        let edit = Edit {
            start: 2,
            end: 4,
            replacement: "\n>".to_string(),
            output_byte_to_input_byte: vec![Some(3), Some(2)],
            moved_delim_len: 1,
        };
        let output = apply_single_edit(input, &edit);

        // Build an input sourcemap that maps the input '>' and '<' to different source columns.
        let mut builder = sourcemap::SourceMapBuilder::new(None);
        builder.add_source("orig.astro");
        // dst (input) position of '>' is line 0 col 2.
        builder.add(0, 2, 10, 20, Some("orig.astro"), None, false);
        // dst (input) position of '<' is line 1 col 0.
        builder.add(1, 0, 10, 100, Some("orig.astro"), None, false);
        let in_map = builder.into_sourcemap();
        let mut buf = Vec::new();
        in_map.to_writer(&mut buf).unwrap();
        let in_map_json = String::from_utf8(buf).unwrap();

        let out_map_json = rewrite_sourcemap(input, &output, &in_map_json, &[edit]).unwrap();
        let out_map = sourcemap::SourceMap::from_slice(out_map_json.as_bytes()).unwrap();

        // Output '>' is at line 1 col 0; should map to src col 20.
        let t_gt = out_map.lookup_token(1, 0).expect("token for moved '>'");
        assert_eq!(t_gt.get_source(), Some("orig.astro"));
        assert_eq!(t_gt.get_src_line(), 10);
        assert_eq!(t_gt.get_src_col(), 20);

        // Output '<' is at line 1 col 1; should map to src col 100 (not 20).
        let t_lt = out_map.lookup_token(1, 1).expect("token for '<'");
        assert_eq!(t_lt.get_source(), Some("orig.astro"));
        assert_eq!(t_lt.get_src_line(), 10);
        assert_eq!(t_lt.get_src_col(), 100);
    }

    /// Basic correctness for line start computation.
    #[test]
    fn compute_line_starts_basic() {
        let s = "a\nbc\nd\n";
        assert_eq!(compute_line_starts(s), vec![0, 2, 5, 7]);
    }

    /// Ensure line start computation works without trailing newline.
    #[test]
    fn compute_line_starts_no_trailing_newline() {
        let s = "a\nb";
        assert_eq!(compute_line_starts(s), vec![0, 2]);
    }

    /// Round-trip test: line/col to byte for valid ranges.
    #[test]
    fn line_col_to_byte_in_range() {
        let s = "ab\ncde\nf";
        let starts = compute_line_starts(s);
        assert_eq!(line_col_to_byte(&starts, 0, 0), Some(0));
        assert_eq!(line_col_to_byte(&starts, 0, 1), Some(1));
        assert_eq!(line_col_to_byte(&starts, 1, 0), Some(3));
        assert_eq!(line_col_to_byte(&starts, 1, 1), Some(4));
        assert_eq!(line_col_to_byte(&starts, 1, 2), Some(5));
        assert_eq!(line_col_to_byte(&starts, 2, 0), Some(7));
    }

    /// Out-of-range lines return `None`.
    #[test]
    fn line_col_to_byte_out_of_range_line() {
        let s = "a\n";
        let starts = compute_line_starts(s);
        assert_eq!(line_col_to_byte(&starts, 2, 0), None);
    }

    /// Round-trip test for `byte_to_line_col` at each line start.
    #[test]
    fn byte_to_line_col_roundtrip_at_line_starts() {
        let s = "ab\ncd\nefg\nhij\n";
        let starts = compute_line_starts(s);
        for (line, &byte) in starts.iter().enumerate() {
            let (l, c) = byte_to_line_col(&starts, byte);
            assert_eq!((l, c), (line, 0));
        }
    }

    /// Behavior test for mid-line and out-of-range byte offsets.
    #[test]
    fn byte_to_line_col_middle_of_line() {
        let s = "ab\ncde\nf";
        let starts = compute_line_starts(s);
        assert_eq!(byte_to_line_col(&starts, 0), (0, 0));
        assert_eq!(byte_to_line_col(&starts, 1), (0, 1));
        assert_eq!(byte_to_line_col(&starts, 2), (0, 2));
        assert_eq!(byte_to_line_col(&starts, 3), (1, 0));
        assert_eq!(byte_to_line_col(&starts, 4), (1, 1));
        assert_eq!(byte_to_line_col(&starts, 5), (1, 2));
        assert_eq!(byte_to_line_col(&starts, 6), (1, 3));
        assert_eq!(byte_to_line_col(&starts, 7), (2, 0));
        // Overflow beyond end of input.
        assert_eq!(byte_to_line_col(&starts, 8), (2, 1));
        assert_eq!(byte_to_line_col(&starts, 9), (2, 2));
        assert_eq!(byte_to_line_col(&starts, 100), (2, 93));
    }

    /// If the input sourcemap contains unmapped tokens, they should remain unmapped.
    #[test]
    fn rewrite_sourcemap_handles_unmapped_input_tokens() {
        // If the input map has unmapped tokens, we should preserve them as unmapped.
        let input = "<a>\n<b>";
        let edit = Edit {
            start: 2,
            end: 4,
            replacement: "\n>".to_string(),
            output_byte_to_input_byte: vec![Some(3), Some(2)],
            moved_delim_len: 1,
        };
        let output = apply_single_edit(input, &edit);

        let mut builder = sourcemap::SourceMapBuilder::new(None);
        // An unmapped token: src_line/src_col u32::MAX with no source.
        builder.add(0, 0, u32::MAX, u32::MAX, None, None, false);
        let in_map = builder.into_sourcemap();
        let mut buf = Vec::new();
        in_map.to_writer(&mut buf).unwrap();
        let in_map_json = String::from_utf8(buf).unwrap();

        let out_map_json = rewrite_sourcemap(input, &output, &in_map_json, &[edit]).unwrap();
        let out_map = sourcemap::SourceMap::from_slice(out_map_json.as_bytes()).unwrap();

        let t = out_map.lookup_token(0, 0).expect("unmapped token");
        assert_eq!(t.get_source(), None);
        assert_eq!(t.get_src_line(), u32::MAX);
        assert_eq!(t.get_src_col(), u32::MAX);
    }

    /// Dedup should prefer mapped entries over unmapped for the same destination.
    #[test]
    fn sort_and_dedup_mappings_prefers_mapped_over_unmapped() {
        let mut mappings = vec![
            Mapping {
                dst_line: 1,
                dst_col: 1,
                src_line: u32::MAX,
                src_col: u32::MAX,
                source: None,
                name: None,
            },
            Mapping {
                dst_line: 1,
                dst_col: 1,
                src_line: 3,
                src_col: 4,
                source: Some("orig.astro"),
                name: None,
            },
        ];

        sort_and_dedup_mappings(&mut mappings);
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].source, Some("orig.astro"));
        assert_eq!(mappings[0].src_line, 3);
        assert_eq!(mappings[0].src_col, 4);
    }

    /// Inserted bytes (`None` origins) should be emitted as unmapped tokens.
    #[test]
    fn create_sourcemap_marks_inserted_bytes_unmapped() {
        // Insert a byte without any input origin.
        let input = "ab";
        let edit = Edit {
            start: 1,
            end: 1,
            replacement: "X".to_string(),
            output_byte_to_input_byte: vec![None],
            moved_delim_len: 0,
        };
        let output = apply_single_edit(input, &edit);
        assert_eq!(output, "aXb");

        let sm_json = create_sourcemap(input, &output, "input.astro", &[edit]).unwrap();
        let sm = sourcemap::SourceMap::from_slice(sm_json.as_bytes()).unwrap();

        let t = sm.lookup_token(0, 1).expect("token for inserted byte");
        assert_eq!(t.get_source(), None);
        assert_eq!(t.get_src_line(), u32::MAX);
        assert_eq!(t.get_src_col(), u32::MAX);
    }

    #[test]
    fn validate_edits_rejects_overlap() {
        let edits = vec![
            Edit {
                start: 1,
                end: 3,
                replacement: "".to_string(),
                output_byte_to_input_byte: Vec::new(),
                moved_delim_len: 0,
            },
            Edit {
                start: 2,
                end: 4,
                replacement: "".to_string(),
                output_byte_to_input_byte: Vec::new(),
                moved_delim_len: 0,
            },
        ];

        let err = validate_edits(10, &edits).unwrap_err();
        assert!(matches!(err, StripError::OverlappingEdits { .. }));
    }

    #[test]
    fn create_sourcemap_uses_utf16_columns_for_unicode_prefix() {
        // Prefix contains Japanese + emoji, so byte columns differ from UTF-16 columns.
        // We rotate ">\n" so the edit start anchor is mid-line.
        let input = "あ🙂<a>\n<b>";
        let start = input.find(">\n").unwrap();
        let end = start + 2;
        let edit = Edit {
            start,
            end,
            replacement: "\n>".to_string(),
            output_byte_to_input_byte: vec![Some(start + 1), Some(start)],
            moved_delim_len: 1,
        };
        let output = apply_single_edit(input, &edit);
        assert_eq!(output, "あ🙂<a\n><b>");

        let sm_json = create_sourcemap(input, &output, "input.astro", &[edit]).unwrap();
        let sm = sourcemap::SourceMap::from_slice(sm_json.as_bytes()).unwrap();

        // At the start of the replacement (right after "あ🙂<a"), the output UTF-16 column is:
        // "あ" (1) + "🙂" (2) + "<" (1) + "a" (1) = 5.
        // That output newline maps back to the original input newline, which comes after the
        // original '>' (so input UTF-16 column is 6).
        let mut found = false;
        for t in sm.tokens() {
            if t.get_dst_line() == 0
                && t.get_dst_col() == 5
                && t.get_source() == Some("input.astro")
                && t.get_src_line() == 0
                && t.get_src_col() == 6
            {
                found = true;
                break;
            }
        }
        assert!(found, "expected a UTF-16 column anchor at line 0 col 5");
    }

    #[test]
    fn rewrite_sourcemap_accepts_utf16_input_columns_with_unicode() {
        let input = "あ🙂<a>\n<b>";
        let start = input.find(">\n").unwrap();
        let end = start + 2;
        let edit = Edit {
            start,
            end,
            replacement: "\n>".to_string(),
            output_byte_to_input_byte: vec![Some(start + 1), Some(start)],
            moved_delim_len: 1,
        };
        let output = apply_single_edit(input, &edit);

        // Build an input sourcemap that uses UTF-16 columns for the generated input.
        // In input, '>' is on line 0, UTF-16 col 5; '<' of <b> is on line 1, col 0.
        let mut builder = sourcemap::SourceMapBuilder::new(None);
        builder.add_source("orig.astro");
        builder.add(0, 5, 10, 20, Some("orig.astro"), None, false);
        builder.add(1, 0, 10, 100, Some("orig.astro"), None, false);
        let in_map = builder.into_sourcemap();
        let mut buf = Vec::new();
        in_map.to_writer(&mut buf).unwrap();
        let in_map_json = String::from_utf8(buf).unwrap();

        let out_map_json = rewrite_sourcemap(input, &output, &in_map_json, &[edit]).unwrap();
        let out_map = sourcemap::SourceMap::from_slice(out_map_json.as_bytes()).unwrap();

        // Output '>' is at line 1 col 0; should map to src col 20.
        let t_gt = out_map.lookup_token(1, 0).expect("token for moved '>'");
        assert_eq!(t_gt.get_source(), Some("orig.astro"));
        assert_eq!(t_gt.get_src_line(), 10);
        assert_eq!(t_gt.get_src_col(), 20);

        // Output '<' is at line 1 col 1; should map to src col 100 (not 20).
        let t_lt = out_map.lookup_token(1, 1).expect("token for '<'");
        assert_eq!(t_lt.get_source(), Some("orig.astro"));
        assert_eq!(t_lt.get_src_line(), 10);
        assert_eq!(t_lt.get_src_col(), 100);
    }
}
