//! UTF-16 column indexing utilities.
//!
//! This module provides a fast mapping between:
//!
//! - absolute **byte offsets** into a UTF-8 `&str`, and
//! - `(line, column)` positions where `column` is measured in **UTF-16 code units**.
//!
//! This matches the de-facto convention used by JavaScript tooling (including many sourcemap
//! consumers), where string indices and columns are typically expressed in UTF-16 code units.
//!
//! Design goals:
//!
//! - Keep the rest of the crate byte-based (tree-sitter model) while emitting sourcemaps in
//!   UTF-16 columns.
//! - Be performant: per-line indexes store checkpoints (every N chars) so conversions avoid
//!   rescanning from the line start.
//! - Be robust: offsets/columns that fall inside multi-byte UTF-8 sequences or inside a surrogate
//!   pair boundary are handled by clamping to the start of the containing Unicode scalar value.

use std::cmp::Ordering;

/// A precomputed UTF-16 column index for an entire string.
///
/// The index is built from `line_starts`, which must contain the byte offsets where each line
/// begins. Typically this comes from scanning for `\n` bytes and pushing `i + 1` for each newline.
///
/// Notes:
///
/// - Lines are considered to end before the line terminator. If the source uses CRLF (`\r\n`),
///   the `\r` is also excluded from the line.
/// - All byte offsets are absolute offsets into the original `&str`.
#[derive(Debug, Clone)]
pub struct Utf16Index<'a> {
    s: &'a str,
    line_starts: Vec<usize>,
    lines: Vec<Utf16LineIndex>,
}

impl<'a> Utf16Index<'a> {
    /// Build an index for `s` given its line starts.
    ///
    /// `line_starts` must be sorted, must start with `0`, and must only contain valid UTF-8
    /// boundaries.
    pub fn new(s: &'a str, line_starts: &[usize]) -> Self {
        let mut lines: Vec<Utf16LineIndex> = Vec::with_capacity(line_starts.len());
        for (i, &start) in line_starts.iter().enumerate() {
            let mut end = match line_starts.get(i + 1).copied() {
                Some(next) => next.saturating_sub(1), // exclude '\n'
                None => s.len(),
            };
            // If the input is CRLF, also exclude the '\r' so it doesn't count toward columns.
            if end > start && s.as_bytes().get(end - 1) == Some(&b'\r') {
                end = end.saturating_sub(1);
            }
            lines.push(Utf16LineIndex::new(s, start, end));
        }
        Self {
            s,
            line_starts: line_starts.to_vec(),
            lines,
        }
    }

    /// Return the 0-based line index that contains `byte`.
    ///
    /// If `byte` is beyond the end of the string, this returns the final line.
    pub fn line_for_byte(&self, byte: usize) -> usize {
        // Find the last line start <= byte.
        match self.line_starts.binary_search_by(|&probe| {
            if probe <= byte {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        }
    }

    /// Convert an absolute byte offset into a `(line, utf16_col)` pair.
    ///
    /// If `byte` points into the middle of a multi-byte UTF-8 sequence, the returned column is
    /// clamped to the start of that Unicode scalar value.
    pub fn byte_to_line_utf16_col(&self, byte: usize) -> (usize, usize) {
        let line = self.line_for_byte(byte);
        let col = self.lines[line].byte_to_utf16_col(self.s, byte);
        (line, col)
    }

    /// Convert a `(line, utf16_col)` pair into an absolute byte offset.
    ///
    /// - Out-of-range `line` returns `None`.
    /// - Columns beyond the line length clamp to the line end.
    /// - If `utf16_col` falls inside a surrogate pair (e.g. in the middle of an emoji), the
    ///   returned byte offset clamps to the start of that code point.
    pub fn line_utf16_col_to_byte(&self, line: usize, utf16_col: usize) -> Option<usize> {
        let li = self.lines.get(line)?;
        Some(li.utf16_col_to_byte(self.s, utf16_col))
    }
}

/// Per-line UTF-16 column index.
///
/// This stores UTF-16 column checkpoints for a single line slice `s[start..end]`.
///
/// The line slice excludes the line terminator (`\n`) and, for CRLF inputs, also excludes the
/// preceding `\r`.
#[derive(Debug, Clone)]
struct Utf16LineIndex {
    /// Absolute byte offset (inclusive) of the line start.
    start: usize,
    /// Absolute byte offset (exclusive) of the line end.
    end: usize,
    /// Sparse checkpoints mapping byte offsets to UTF-16 columns.
    checkpoints: Vec<Utf16Checkpoint>,
}

impl Utf16LineIndex {
    /// Number of Unicode scalar values between checkpoints.
    ///
    /// Higher values reduce memory but increase worst-case scan length for a single conversion.
    const CHECKPOINT_STRIDE_CHARS: usize = 64;

    /// Build a per-line index for `s[start..end]`.
    ///
    /// `start` and `end` are absolute byte offsets into `s`. They must lie on UTF-8 boundaries.
    fn new(s: &str, start: usize, end: usize) -> Self {
        // `end` is exclusive, and may point at a '\n' byte (i.e. the newline itself is not part
        // of the line slice). This matches typical line/column semantics.
        let mut checkpoints: Vec<Utf16Checkpoint> = Vec::new();
        checkpoints.push(Utf16Checkpoint {
            byte: start,
            utf16_col: 0,
        });

        let mut utf16_col: usize = 0;
        let mut char_count: usize = 0;

        let line = &s[start..end];
        for (rel, ch) in line.char_indices() {
            utf16_col += ch.len_utf16();
            char_count += 1;

            if char_count.is_multiple_of(Self::CHECKPOINT_STRIDE_CHARS) {
                let next_byte = start + rel + ch.len_utf8();
                checkpoints.push(Utf16Checkpoint {
                    byte: next_byte,
                    utf16_col,
                });
            }
        }

        // Always include the line end boundary.
        if checkpoints.last().map(|c| c.byte).unwrap_or(start) != end {
            checkpoints.push(Utf16Checkpoint {
                byte: end,
                utf16_col,
            });
        }

        Self {
            start,
            end,
            checkpoints,
        }
    }

    /// Return the UTF-16 length of this line.
    fn utf16_len(&self) -> usize {
        self.checkpoints.last().map(|c| c.utf16_col).unwrap_or(0)
    }

    /// Convert an absolute byte offset into a UTF-16 column within this line.
    ///
    /// If `abs_byte` lies inside a UTF-8 code unit sequence, the returned column clamps to the
    /// start of the containing Unicode scalar value.
    fn byte_to_utf16_col(&self, s: &str, abs_byte: usize) -> usize {
        if abs_byte <= self.start {
            return 0;
        }
        let clamped = abs_byte.min(self.end);

        let cp_idx = match self.checkpoints.binary_search_by(|c| c.byte.cmp(&clamped)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        let cp = self.checkpoints[cp_idx];

        let mut cur_byte = cp.byte;
        let mut cur_utf16 = cp.utf16_col;

        let line = &s[self.start..self.end];

        while cur_byte < clamped {
            let rel = cur_byte - self.start;
            if rel >= line.len() {
                break;
            }
            let ch = line[rel..].chars().next().unwrap();
            let ch_len = ch.len_utf8();
            let next_byte = cur_byte + ch_len;
            if next_byte <= clamped {
                cur_utf16 += ch.len_utf16();
                cur_byte = next_byte;
            } else {
                // `clamped` points into the middle of this UTF-8 sequence; treat it as the start
                // of the character for column purposes.
                break;
            }
        }

        cur_utf16
    }

    /// Convert a UTF-16 column within this line into an absolute byte offset.
    ///
    /// Columns beyond the line's UTF-16 length clamp to the end of the line.
    ///
    /// If `utf16_col` falls inside a surrogate pair boundary (e.g. a value of 1 inside an emoji
    /// that occupies 2 UTF-16 code units), this clamps to the start of that code point.
    fn utf16_col_to_byte(&self, s: &str, utf16_col: usize) -> usize {
        if utf16_col == 0 {
            return self.start;
        }
        if utf16_col >= self.utf16_len() {
            return self.end;
        }

        // Find the last checkpoint with utf16_col <= target.
        let cp_idx = match self
            .checkpoints
            .binary_search_by(|c| c.utf16_col.cmp(&utf16_col))
        {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        };
        let cp = self.checkpoints[cp_idx];

        let mut cur_byte = cp.byte;
        let mut cur_utf16 = cp.utf16_col;

        let line = &s[self.start..self.end];
        while cur_utf16 < utf16_col && cur_byte < self.end {
            let rel = cur_byte - self.start;
            if rel >= line.len() {
                break;
            }
            let ch = line[rel..].chars().next().unwrap();
            let u16_len = ch.len_utf16();
            if cur_utf16 + u16_len > utf16_col {
                // Target falls inside a surrogate pair boundary (e.g. emoji). Clamp to the start
                // of the code point.
                break;
            }
            cur_utf16 += u16_len;
            cur_byte += ch.len_utf8();
        }

        cur_byte
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A sparse checkpoint inside a line.
///
/// Stores the absolute byte offset and the corresponding UTF-16 column.
struct Utf16Checkpoint {
    /// Absolute byte offset into the original string.
    byte: usize,
    /// UTF-16 column at `byte`.
    utf16_col: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compute line start byte offsets for `s`.
    fn compute_line_starts(s: &str) -> Vec<usize> {
        let mut starts = vec![0usize];
        for (i, b) in s.as_bytes().iter().copied().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    /// Naive (scan-from-start) conversion of a line-relative byte offset to a UTF-16 column.
    fn naive_byte_to_utf16_col(line: &str, rel_byte: usize) -> usize {
        let clamped = rel_byte.min(line.len());
        let mut col = 0usize;
        let mut cur = 0usize;
        while cur < clamped {
            let ch = line[cur..].chars().next().unwrap();
            let next = cur + ch.len_utf8();
            if next <= clamped {
                col += ch.len_utf16();
                cur = next;
            } else {
                break;
            }
        }
        col
    }

    /// Naive (scan-from-start) conversion of a UTF-16 column to a line-relative byte offset.
    ///
    /// This mirrors the clamping behavior used by the indexed implementation.
    fn naive_utf16_col_to_byte(line: &str, utf16_col: usize) -> usize {
        let mut cur_u16 = 0usize;
        let mut cur_byte = 0usize;
        while cur_byte < line.len() {
            let ch = line[cur_byte..].chars().next().unwrap();
            let u16 = ch.len_utf16();
            if cur_u16 + u16 > utf16_col {
                break;
            }
            cur_u16 += u16;
            cur_byte += ch.len_utf8();
            if cur_u16 == utf16_col {
                break;
            }
        }
        cur_byte
    }

    #[test]
    /// ASCII-only strings have identical byte and UTF-16 columns.
    fn ascii_roundtrips() {
        let s = "abc";
        let starts = compute_line_starts(s);
        let idx = Utf16Index::new(s, &starts);

        assert_eq!(idx.byte_to_line_utf16_col(0), (0, 0));
        assert_eq!(idx.byte_to_line_utf16_col(1), (0, 1));
        assert_eq!(idx.byte_to_line_utf16_col(3), (0, 3));

        assert_eq!(idx.line_utf16_col_to_byte(0, 0), Some(0));
        assert_eq!(idx.line_utf16_col_to_byte(0, 2), Some(2));
        assert_eq!(idx.line_utf16_col_to_byte(0, 99), Some(3));
        assert_eq!(idx.line_utf16_col_to_byte(1, 0), None);
    }

    #[test]
    /// Japanese characters are multi-byte in UTF-8 but single-unit in UTF-16.
    fn japanese_utf8_bytes_map_to_single_utf16_units() {
        let s = "あい"; // each is 3 bytes in UTF-8, 1 UTF-16 unit
        let starts = compute_line_starts(s);
        let idx = Utf16Index::new(s, &starts);

        assert_eq!(idx.byte_to_line_utf16_col(0), (0, 0));
        assert_eq!(idx.byte_to_line_utf16_col(3), (0, 1));
        assert_eq!(idx.byte_to_line_utf16_col(6), (0, 2));

        // Mid-byte offsets clamp to the character start.
        assert_eq!(idx.byte_to_line_utf16_col(1), (0, 0));
        assert_eq!(idx.byte_to_line_utf16_col(4), (0, 1));

        assert_eq!(idx.line_utf16_col_to_byte(0, 0), Some(0));
        assert_eq!(idx.line_utf16_col_to_byte(0, 1), Some(3));
        assert_eq!(idx.line_utf16_col_to_byte(0, 2), Some(6));
    }

    #[test]
    /// Emoji are represented as surrogate pairs in UTF-16 (2 code units).
    fn emoji_is_two_utf16_units_and_clamps_inside_surrogate_pair() {
        let s = "🙂"; // 4 bytes, 2 UTF-16 units
        let starts = compute_line_starts(s);
        let idx = Utf16Index::new(s, &starts);

        assert_eq!(idx.byte_to_line_utf16_col(0), (0, 0));
        assert_eq!(idx.byte_to_line_utf16_col(4), (0, 2));
        // Mid-byte clamps to start.
        assert_eq!(idx.byte_to_line_utf16_col(2), (0, 0));

        assert_eq!(idx.line_utf16_col_to_byte(0, 0), Some(0));
        // Inside surrogate pair => clamp to start of code point.
        assert_eq!(idx.line_utf16_col_to_byte(0, 1), Some(0));
        assert_eq!(idx.line_utf16_col_to_byte(0, 2), Some(4));
    }

    #[test]
    /// Mixed ASCII and emoji should round-trip cleanly.
    fn mixed_ascii_emoji_roundtrips() {
        let s = "a🙂b";
        let starts = compute_line_starts(s);
        let idx = Utf16Index::new(s, &starts);

        // a(1) + 🙂(2) + b(1)
        assert_eq!(idx.byte_to_line_utf16_col(0), (0, 0));
        assert_eq!(idx.byte_to_line_utf16_col(1), (0, 1));
        assert_eq!(idx.byte_to_line_utf16_col(5), (0, 3));
        assert_eq!(idx.byte_to_line_utf16_col(6), (0, 4));

        assert_eq!(idx.line_utf16_col_to_byte(0, 1), Some(1));
        assert_eq!(idx.line_utf16_col_to_byte(0, 3), Some(5));
        assert_eq!(idx.line_utf16_col_to_byte(0, 4), Some(6));
    }

    #[test]
    /// Multi-line conversion uses the provided line starts and stays line-relative.
    fn multi_line_positions() {
        let s = "a🙂\nあb";
        let starts = compute_line_starts(s);
        let idx = Utf16Index::new(s, &starts);

        // newline is at byte 5, second line starts at 6
        assert_eq!(starts, vec![0, 6]);

        assert_eq!(idx.byte_to_line_utf16_col(0), (0, 0));
        assert_eq!(idx.byte_to_line_utf16_col(1), (0, 1));
        assert_eq!(idx.byte_to_line_utf16_col(5), (0, 3));
        assert_eq!(idx.byte_to_line_utf16_col(6), (1, 0));

        assert_eq!(idx.line_utf16_col_to_byte(0, 3), Some(5));
        assert_eq!(idx.line_utf16_col_to_byte(1, 0), Some(6));
        assert_eq!(idx.line_utf16_col_to_byte(1, 1), Some(9)); // 'あ' is 3 bytes in UTF-8
        assert_eq!(idx.line_utf16_col_to_byte(1, 2), Some(10));
    }

    #[test]
    /// For CRLF inputs, the '\r' should not contribute to columns.
    fn crlf_excludes_carriage_return_from_columns() {
        let s = "a🙂\r\nあb";
        let starts = compute_line_starts(s);
        let idx = Utf16Index::new(s, &starts);

        // First line: "a🙂" => utf16 len is 3.
        // Byte offsets: a(0..1), 🙂(1..5), \r(5), \n(6)
        assert_eq!(idx.byte_to_line_utf16_col(5), (0, 3));
        assert_eq!(idx.byte_to_line_utf16_col(6), (0, 3));
        assert_eq!(idx.byte_to_line_utf16_col(7), (1, 0));

        // Column beyond end clamps to line end.
        assert_eq!(idx.line_utf16_col_to_byte(0, 99), Some(5));
    }

    #[test]
    /// The checkpointed implementation matches a naive scan on a long mixed line.
    fn checkpointed_matches_naive_on_long_mixed_line() {
        // Create a long line to exercise checkpoint logic.
        let mut line = String::new();
        for _ in 0..200 {
            line.push('a');
            line.push('🙂');
            line.push('あ');
        }
        let s = format!("{line}\nnext");
        let starts = compute_line_starts(&s);
        let idx = Utf16Index::new(&s, &starts);

        // Validate various byte positions on the first line.
        let line_end = s.find('\n').unwrap();
        let line_slice = &s[..line_end];
        for rel in [
            0usize,
            1,
            2,
            3,
            4,
            5,
            10,
            63,
            64,
            65,
            256,
            999,
            line_slice.len(),
        ] {
            let abs = rel.min(line_slice.len());
            let (_, got) = idx.byte_to_line_utf16_col(abs);
            let want = naive_byte_to_utf16_col(line_slice, abs);
            assert_eq!(got, want, "byte->utf16 mismatch at rel byte {abs}");
        }

        // Validate various UTF-16 columns on the first line.
        let max_u16 = line_slice.chars().map(|c| c.len_utf16()).sum::<usize>();
        for col in [
            0usize,
            1,
            2,
            3,
            4,
            5,
            64,
            65,
            256,
            999,
            max_u16,
            max_u16 + 10,
        ] {
            let got = idx.line_utf16_col_to_byte(0, col).unwrap();
            let want = naive_utf16_col_to_byte(line_slice, col.min(max_u16));
            assert_eq!(got, want, "utf16->byte mismatch at col {col}");
        }
    }
}
