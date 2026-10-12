use std::io::{self, Write};
use std::process::{Command, ExitStatus};

/// Errors raised while preparing or running the Markdown conversion.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("preparing Pandoc input or filter: {0}")]
    Io(#[from] io::Error),
    #[error("running pandoc: {0}")]
    PandocSpawn(#[source] io::Error),
    #[error("pandoc exited with {status}: {stderr}")]
    PandocFailed { status: ExitStatus, stderr: String },
    #[error("pandoc output is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

// TODO use markdown-rs, pulldown-cmark or comrak
/// Convert GFM to HTML that can be pasted into Slack
///
/// Requires `pandoc` on PATH. Nested lists are flattened using the lua filter, preserving indentation and ordered-list start numbers.
///
/// Returns [`Error`] for temporary-file I/O, Pandoc launch or conversion
/// failures, or invalid UTF-8 output.
pub fn to_html(markdown: &str) -> Result<String, Error> {
    let mut filter = tempfile::Builder::new().suffix(".lua").tempfile()?;
    filter.write_all(include_bytes!("slack-lists.lua"))?;
    let mut input = tempfile::NamedTempFile::new()?;
    input.write_all(markdown.as_bytes())?;
    let output = Command::new("pandoc")
        .args(["--from=gfm", "--to=html", "--lua-filter"])
        .arg(filter.path())
        .arg(input.path())
        .output()
        .map_err(Error::PandocSpawn)?;
    if !output.status.success() {
        return Err(Error::PandocFailed {
            status: output.status,
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8(output.stdout)?)
}
