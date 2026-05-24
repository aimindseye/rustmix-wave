//! Library title layout helper.
//!
//! This helper is pure and side-effect free. It does not choose a title source.
//! It only normalizes display bytes for a fixed-width library row.

#![allow(dead_code)]

pub const LIBRARY_TITLE_LAYOUT_HELPER_MARKER: &str = "x4-library-title-layout-helper-ok";

pub const LIBRARY_TITLE_MAX_BYTES: usize = 42;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryTitlePolishOutcome {
    Empty,
    Copied,
    Trimmed,
    Ellipsized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LibraryTitlePolishReport {
    pub outcome: LibraryTitlePolishOutcome,
    pub len: usize,
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_separator(byte: u8) -> bool {
    matches!(byte, b'_' | b'-')
}

pub fn polish_library_title<'a>(
    input: &[u8],
    output: &'a mut [u8],
) -> (&'a [u8], LibraryTitlePolishReport) {
    if output.is_empty() {
        return (
            &output[..0],
            LibraryTitlePolishReport {
                outcome: LibraryTitlePolishOutcome::Empty,
                len: 0,
            },
        );
    }

    let mut start = 0;
    let mut end = input.len();

    while start < end && is_space(input[start]) {
        start += 1;
    }
    while end > start && is_space(input[end - 1]) {
        end -= 1;
    }

    if start >= end {
        return (
            &output[..0],
            LibraryTitlePolishReport {
                outcome: LibraryTitlePolishOutcome::Empty,
                len: 0,
            },
        );
    }

    let mut written = 0usize;
    let mut previous_space = false;
    let max = core::cmp::min(output.len(), LIBRARY_TITLE_MAX_BYTES);

    let mut index = start;
    while index < end && written < max {
        let byte = input[index];
        let normalized = if is_separator(byte) { b' ' } else { byte };
        let normalized_space = is_space(normalized);

        if normalized_space {
            if !previous_space && written > 0 {
                output[written] = b' ';
                written += 1;
            }
            previous_space = true;
        } else {
            output[written] = normalized;
            written += 1;
            previous_space = false;
        }

        index += 1;
    }

    while written > 0 && output[written - 1] == b' ' {
        written -= 1;
    }

    let mut outcome = if index < end {
        LibraryTitlePolishOutcome::Trimmed
    } else {
        LibraryTitlePolishOutcome::Copied
    };

    if index < end && written >= 3 {
        output[written - 3] = b'.';
        output[written - 2] = b'.';
        output[written - 1] = b'.';
        outcome = LibraryTitlePolishOutcome::Ellipsized;
    }

    (
        &output[..written],
        LibraryTitlePolishReport {
            outcome,
            len: written,
        },
    )
}

pub fn library_title_layout_helper_marker() -> &'static str {
    LIBRARY_TITLE_LAYOUT_HELPER_MARKER
}
