// The MIT License (MIT)

// Copyright (c) 2026 AnonymousDapper

use std::path::{Path, PathBuf};

use unicode_normalization::UnicodeNormalization;

#[allow(unused_imports)]
use crate::logging::{debug, error, info, trace, warn};

use crate::frontend::token::{Index, Span};

pub struct SourceFile {
    path: String,
    name: String,
    length: Index,
    source: Vec<u8>,
    line_starts: Vec<Index>,
    line_lengths: Vec<Index>,
}

pub struct ConstantSource {
    length: Index,
    source: Vec<u8>,
    line_starts: Vec<Index>,
    line_lengths: Vec<Index>,
}

pub trait Source {
    fn source_id(&self) -> &str;

    fn length(&self) -> Index;

    fn line_count(&self) -> Index;

    fn line_starts(&self) -> &Vec<Index>;

    fn line_lengths(&self) -> &Vec<Index>;

    fn line_column(&self, offset: Index) -> (Index, Index);

    fn line_span(&self, line: Index) -> Span;

    fn read_at(&self, pos: Index) -> Option<u8>;

    fn read_span(&self, span: Span) -> &[u8];

    fn read_to_end(&self, index: Index) -> &[u8];

    fn read_text_span(&self, span: Span) -> &str {
        unsafe { std::str::from_utf8_unchecked(self.read_span(span)) }
    }

    fn text(&self) -> &str {
        // as long as we validate/normalize UTF8 when loading, this is safe
        unsafe { std::str::from_utf8_unchecked(self.read_to_end(0)) }
    }
}

impl Source for SourceFile {
    fn source_id(&self) -> &str {
        &self.name
    }

    fn length(&self) -> Index {
        self.length
    }

    fn line_count(&self) -> Index {
        self.line_starts.len() as Index
    }

    fn line_starts(&self) -> &Vec<Index> {
        &self.line_starts
    }

    fn line_lengths(&self) -> &Vec<Index> {
        &self.line_lengths
    }

    fn line_column(&self, offset: Index) -> (Index, Index) {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        let column = offset - self.line_starts[line];

        (line as Index, column)
    }

    fn line_span(&self, line: Index) -> Span {
        let us_line = line as usize;

        let start = self.line_starts[us_line];

        let end = self
            .line_starts
            .get(us_line + 1)
            .copied()
            .unwrap_or(self.length);

        Span::new(start, end)
    }

    fn read_at(&self, pos: Index) -> Option<u8> {
        self.source.get(pos as usize).copied()
    }

    fn read_span(&self, span: Span) -> &[u8] {
        &self.source[span.range()]
    }

    fn read_to_end(&self, index: Index) -> &[u8] {
        &self.source[index as usize..]
    }
}

impl SourceFile {
    pub fn from_path(path: PathBuf) -> Result<SourceFile, String> {
        match read_from_path(&path) {
            Ok(text) => {
                let (line_starts, line_lengths) = count_line_data(&text);

                // let file_name = path
                //     .file_name()
                //     .map(|o| o.to_str().unwrap_or("<invalid>"))
                //     .unwrap_or("<unknown>");
                let file_name = path.file_name().unwrap().display();

                // let file_path = path
                //     .parent()
                //     .map(|p| p.to_str().unwrap_or("<invalid>"))
                //     .unwrap_or("<unknown>");
                let file_path = path.parent().unwrap().display();

                //trace!(target:"SourceFile::from_path", "{}", path.file_name().unwrap().display());

                if let Ok(t_len) = std::convert::TryInto::<Index>::try_into(text.len()) {
                    Ok(SourceFile {
                        path: file_path.to_string(),
                        name: file_name.to_string(),
                        length: t_len,
                        source: text.into_bytes(),
                        line_starts,
                        line_lengths,
                    })
                } else {
                    Err(String::from(
                        "Internal Compiler Error: source length is too large for Index type",
                    ))
                }
            }
            Err(e) => Err(format!(
                "Failed to read source file '{}': {}",
                path.display(),
                e
            )),
        }
    }
}

impl Source for ConstantSource {
    fn source_id(&self) -> &str {
        "<constant>"
    }

    fn length(&self) -> Index {
        self.length
    }

    fn line_count(&self) -> Index {
        self.line_starts.len() as Index
    }

    fn line_starts(&self) -> &Vec<Index> {
        &self.line_starts
    }

    fn line_lengths(&self) -> &Vec<Index> {
        &self.line_lengths
    }

    fn line_column(&self, offset: Index) -> (Index, Index) {
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };

        let column = offset - self.line_starts[line];

        (line as Index, column)
    }

    fn line_span(&self, line: Index) -> Span {
        let us_line = line as usize;

        let start = self.line_starts[us_line];

        let end = self
            .line_starts
            .get(us_line + 1)
            .copied()
            .unwrap_or(self.length);

        Span::new(start, end)
    }

    fn read_at(&self, pos: Index) -> Option<u8> {
        self.source.get(pos as usize).copied()
    }

    fn read_span(&self, span: Span) -> &[u8] {
        &self.source[span.range()]
    }

    fn read_to_end(&self, index: Index) -> &[u8] {
        &self.source[index as usize..]
    }
}

impl ConstantSource {
    pub fn new(input: String) -> Self {
        let (line_starts, line_lengths) = count_line_data(&input);
        ConstantSource {
            length: input.len() as Index,
            source: input.into_bytes(),
            line_starts,
            line_lengths,
        }
    }
}
/// read_from_path
///
/// Takes a Path-like and attempt to read as a String
/// returning any error value as a string.
/// This function also normalizes Unicode input to NFC.
pub fn read_from_path<P: AsRef<Path>>(path: P) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            let nfc_normed: String = s.nfc().collect();

            let mut index = 0;

            // Unicode BOM stripping
            if nfc_normed[0..3] == *"\u{ffef}" {
                index = 3;
            }

            // Shebang stripping
            while nfc_normed[index..index + 2] == *"#!" {
                if let Some(nl) = s.find('\n') {
                    index = nl;
                } else {
                    break;
                }
            }

            // CRLF normalization
            Ok(nfc_normed[index..].replace("\r\n", "\n"))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn count_line_data(source: &str) -> (Vec<Index>, Vec<Index>) {
    let mut line_starts = vec![0];
    let mut line_lengths = vec![];

    // yields line1 \n, line2 \n, ...
    for line in source.split_inclusive('\n') {
        let length = line.len() as Index; // usize to u32 cast
        let last_start = line_starts.last().unwrap();

        line_lengths.push(length);
        line_starts.push(last_start + length);
    }

    // trim the last line start
    line_starts.pop();

    (line_starts, line_lengths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_source() {
        let src = String::from("foo bar\nbar foo\n.\n");

        let source = ConstantSource::new(src);

        assert_eq!(source.length(), 18);

        assert_eq!(source.read_to_end(0), b"foo bar\nbar foo\n.\n");

        assert_eq!(source.line_starts(), &vec![0, 8, 16]);

        assert_eq!(source.line_lengths(), &vec![8, 8, 2]);
    }
}
