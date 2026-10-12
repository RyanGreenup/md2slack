use std::io::{self, Write};
use std::process::Command;

// TODO use markdown-rs, pulldown-cmark or comrak
/// Convert GFM to HTML that can be pasted into Slack
///
/// Requires `pandoc` on PATH. Nested lists are flattened using the lua filter, preserving indentation and ordered-list start numbers.
pub fn to_html(markdown: &str) -> io::Result<String> {
    let mut filter = tempfile::Builder::new().suffix(".lua").tempfile()?;
    filter.write_all(include_bytes!("slack-lists.lua"))?;
    let mut input = tempfile::NamedTempFile::new()?;
    input.write_all(markdown.as_bytes())?;
    let output = Command::new("pandoc")
        .args(["--from=gfm", "--to=html", "--lua-filter"])
        .arg(filter.path())
        .arg(input.path())
        .output()
        // TODO use thiserror
        .map_err(|error| io::Error::new(error.kind(), format!("running pandoc: {error}")))?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "pandoc exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}
