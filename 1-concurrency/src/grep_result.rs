use std::{path::PathBuf, ops::Range, fmt::{Display, Formatter}};

/// This structure represents the matches that the tool found in **a single file**.
/// It implements `Display`, so it can be pretty-printed.
/// This struct and the `Display` trait implementation do NOT need to be edited.
pub struct GrepResult {
    /// the path of the search result
    pub path: PathBuf,

    /// the contents of that file
    pub content: Vec<u8>,

    /// which ranges in the file match the filter.
    /// A file may contain more than one match. Each match is a Range,
    /// which is a start and end byte offset in the content field.
    pub ranges: Vec<Range<usize>>,

    /// The index of this search result (ie. a counter of how many files have had a match before this
    /// one). Note that this count must always increase as the results are printed.
    pub search_ctr: usize,
}

impl Display for GrepResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        const MAX_CONTEXT: usize= 70;

        if self.ranges.is_empty() {
            return Ok(());
        }

        writeln!(f, ">>> (#{}) {:?}", self.search_ctr, self.path)?;
        for range in &self.ranges {
            // Find the index of the first byte before the range that is a newline character, plus one.
            let mut ctx_start = (0..range.start)
                .rev()
                .find(|p| self.content[*p] == b'\n' || self.content[*p] == b'\r')
                .map_or(0, |p| p + 1);

            // Find the index of the first byte after the range that is a newline character (not minus one because it is exclusive)
            let mut ctx_end = (range.end..self.content.len())
                .find(|p| self.content[*p] == b'\n' || self.content[*p] == b'\r')
                .unwrap_or(self.content.len());

            // if the context is too large, reduce its size
            if ctx_start + MAX_CONTEXT < range.start {
                ctx_start = range.start - MAX_CONTEXT;
            }
            if ctx_end > range.end + MAX_CONTEXT {
                ctx_end = range.end + MAX_CONTEXT;
            }

            // Finally, print the result
            writeln!(
                f,
                "{}",
                String::from_utf8_lossy(&self.content[ctx_start..ctx_end])
            )?;
            // Print ^^^^ underneath matched part
            writeln!(
                f,
                "{}{}{}",
                " ".repeat(range.start - ctx_start),
                "^".repeat(range.end - range.start),
                " ".repeat(ctx_end - range.end)
            )?;
        }

        Ok(())
    }
}
