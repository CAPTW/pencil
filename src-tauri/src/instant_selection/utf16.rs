use super::{EngineError, EngineErrorCode};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Utf16Range {
    pub start_utf16: usize,
    pub end_utf16: usize,
}

impl Utf16Range {
    pub const fn new(start_utf16: usize, end_utf16: usize) -> Self {
        Self {
            start_utf16,
            end_utf16,
        }
    }

    pub(crate) const fn intersects(self, other: Self) -> bool {
        self.start_utf16 < other.end_utf16 && other.start_utf16 < self.end_utf16
    }
}

pub fn utf16_len(source: &str) -> usize {
    source.encode_utf16().count()
}

fn byte_boundary_for_utf16(source: &str, offset: usize) -> Result<usize, EngineError> {
    if offset == 0 {
        return Ok(0);
    }

    let mut utf16_offset = 0usize;
    for (byte_start, character) in source.char_indices() {
        utf16_offset = utf16_offset.saturating_add(character.len_utf16());
        if utf16_offset == offset {
            return Ok(byte_start + character.len_utf8());
        }
        if utf16_offset > offset {
            return Err(EngineError::new(EngineErrorCode::SurrogateSplit));
        }
    }

    Err(EngineError::new(EngineErrorCode::InvalidUtf16Range))
}

pub fn utf16_range_to_byte_range(
    source: &str,
    range: Utf16Range,
) -> Result<Range<usize>, EngineError> {
    let source_utf16_length = utf16_len(source);
    if range.start_utf16 >= range.end_utf16 || range.end_utf16 > source_utf16_length {
        return Err(EngineError::new(EngineErrorCode::InvalidUtf16Range));
    }
    let start = byte_boundary_for_utf16(source, range.start_utf16)?;
    let end = byte_boundary_for_utf16(source, range.end_utf16)?;
    Ok(start..end)
}

pub(crate) fn byte_range_to_utf16_range(
    source: &str,
    start_byte: usize,
    end_byte: usize,
) -> Result<Utf16Range, EngineError> {
    if start_byte >= end_byte
        || end_byte > source.len()
        || !source.is_char_boundary(start_byte)
        || !source.is_char_boundary(end_byte)
    {
        return Err(EngineError::new(EngineErrorCode::InvalidUtf16Range));
    }
    Ok(Utf16Range::new(
        source[..start_byte].encode_utf16().count(),
        source[..end_byte].encode_utf16().count(),
    ))
}
