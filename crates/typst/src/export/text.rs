use ecow::{eco_format, EcoString};
use serde::{Deserialize, Serialize};

use crate::{diag::StrResult, model::Content, PathResolver, World};

/// Text content with span information
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SpannedText {
    /// List of source file paths
    pub sources: Vec<String>,
    /// Plain text content of the file
    pub content: EcoString,
    /// Mapping from content offset to source range in Base64 VLQs format
    /// The source range is encoded as a tuple of 7 values:
    /// - start of the text range (offset group)
    /// - end of the text range (offset group)
    /// - file id, known as the index into the sources vector
    /// - start line of the source range (line group)
    /// - start column of the source range (column group)
    /// - end line of the source range (line group)
    /// - end column of the source range (column group)
    ///
    /// The numbers of each group are encoded as deltas to the previous value.
    ///
    /// Example:
    /// ```typ
    /// #let txt = "Hello! world."
    /// #txt
    ///
    /// #txt
    /// ```
    ///
    /// Then the plain text content is:
    /// ```text
    /// Hello! world.Hello! world.
    /// ```
    ///
    /// And the mappings are:
    /// ```
    /// // content offset: 0..13
    /// // file id: 0
    /// // source start: 2:1
    /// // source end: 2:4
    /// 0, 13, 0, 2, 1, 0, 3,
    /// // content offset: 13..26
    /// // file id: 0
    /// // source start: 4:1
    /// // source end: 4:4
    /// 0, 13, 0, 2, -3, 0, 3,
    /// // content offset: 26..27
    /// // file id: 0
    /// // source start: 4:4
    /// // source end: 5:0
    /// 0, 1, 0, 0, 0, 1, -4
    /// ```
    pub mappings: String,
}

/// Export the content as text with span information in json format
pub fn spanned_text(
    world: &dyn World,
    pr: &dyn PathResolver,
    src: &Content,
) -> StrResult<String> {
    let (content, mappings) = src.text_with_spans();
    let mut text = SpannedText { content, ..SpannedText::default() };

    text.mappings.reserve(mappings.len());

    let mut file_mappings = indexmap::IndexSet::new();

    let mut rng_diff: i64 = 0;
    let mut line_diff: i64 = 0;
    let mut column_diff: i64 = 0;

    let mut mapping_buf = std::io::BufWriter::new(vec![]);

    for (text_rng, span) in mappings {
        // Get source information
        let Some((id, src)) =
            span.id().and_then(|id| Some(id).zip(world.source(id).ok()))
        else {
            continue;
        };
        let Some(rng) = src.range(span) else {
            continue;
        };

        // Allocate file id
        let (fid, inserted) = file_mappings.insert_full(id);
        if inserted {
            let Some(path) = pr.resolve_path(id) else {
                continue;
            };

            text.sources.push(format!("{}", path.display()));
        }

        // Get line and column information
        let sl = src.byte_to_line(rng.start);
        let sc = src.byte_to_column(rng.start);
        let el = src.byte_to_line(rng.end);
        let ec = src.byte_to_column(rng.end);
        let Some((((sl, sc), el), ec)) = sl.zip(sc).zip(el).zip(ec) else {
            continue;
        };

        // Encode mapping

        // Start and end of the text range
        let st = text_rng.start as i64;
        vlq::encode(st - rng_diff, &mut mapping_buf).unwrap();
        rng_diff = st;
        let ed = text_rng.end as i64;
        vlq::encode(ed - rng_diff, &mut mapping_buf).unwrap();
        rng_diff = ed;

        // File id
        vlq::encode(fid as i64, &mut mapping_buf).unwrap();

        // Line and column of the source start
        vlq::encode(sl as i64 - line_diff, &mut mapping_buf).unwrap();
        line_diff = sl as i64;
        vlq::encode(sc as i64 - column_diff, &mut mapping_buf).unwrap();
        column_diff = sc as i64;

        // Line and column of the source end
        vlq::encode(el as i64 - line_diff, &mut mapping_buf).unwrap();
        line_diff = el as i64;
        vlq::encode(ec as i64 - column_diff, &mut mapping_buf).unwrap();
        column_diff = ec as i64;
    }

    // Always utf-8 string
    text.mappings = String::from_utf8(mapping_buf.into_inner().unwrap()).unwrap();

    serde_json::to_string(&text).map_err(|e| eco_format!("{}", e))
}

#[cfg(test)]
mod tests {
    #[test]
    fn decode_vlq() {
        let data = b"AaAECAGAaAEHAGACAAACJ";

        let mut input = data.iter().cloned();
        let mut decoded = vec![];
        while let Ok(val) = vlq::decode(&mut input) {
            decoded.push(val);
        }

        assert_eq!(
            decoded,
            vec![0, 13, 0, 2, 1, 0, 3, 0, 13, 0, 2, -3, 0, 3, 0, 1, 0, 0, 0, 1, -4]
        );
    }
}
