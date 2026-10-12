use clap::{ArgGroup, Parser};
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

/// Convert GFM Markdown to HTML that can be pasted into Slack.
#[derive(Parser)]
#[command(version, group(ArgGroup::new("input").args(["file", "stdin", "paste"]).multiple(false)))]
#[command(group(ArgGroup::new("destination").args(["output", "copy"]).required(true).multiple(false)))]
#[command(
    after_help = "Requires pandoc on PATH. Clipboard operations require a Wayland session and wl-clipboard.\n\nExamples:\n  md2slack notes.md -o notes.html\n  cat notes.md | md2slack --copy\n  md2slack --paste -o notes.html\n  md2slack --paste --copy\n\nHTML is a fragment for rich-text pasting, not Slack mrkdwn. Raw HTML in the input is preserved by Pandoc."
)]
struct Args {
    /// Read Markdown from a file (use - for stdin).
    file: Option<PathBuf>,
    /// Read Markdown from stdin (default when no input is specified).
    #[arg(long)]
    stdin: bool,
    // TODO use a clipboard crate
    /// Read Markdown text from the Wayland clipboard using wl-paste.
    #[arg(long)]
    paste: bool,
    /// Write HTML to a file.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    /// Copy HTML to the Wayland clipboard using wl-copy.
    #[arg(long)]
    copy: bool,
}

// TODO use anyhow
fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let markdown = if args.paste {
        let output = Command::new("wl-paste")
            .args(["--no-newline", "--type", "text"])
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "wl-paste exited with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )
            .into());
        }
        String::from_utf8(output.stdout)?
    } else if let Some(path) = args.file.filter(|path| path.as_os_str() != "-") {
        fs::read_to_string(path)?
    } else {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        text
    };
    let html = md2slack::to_html(&markdown)?;
    if let Some(path) = args.output {
        fs::write(path, html)?;
    } else {
        let mut child = Command::new("wl-copy")
            .args(["--type", "text/html"])
            .stdin(Stdio::piped())
            .spawn()?;
        let result = child
            .stdin
            .take()
            .expect("piped stdin")
            .write_all(html.as_bytes());
        let status = child.wait()?;
        if !status.success() {
            return Err(format!("wl-copy exited with {status}").into());
        }
        result?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("md2slack: {error}");
            ExitCode::FAILURE
        }
    }
}
