use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Parser};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use wl_clipboard_rs::{copy, paste};

/// Convert GFM Markdown to HTML that can be pasted into Slack.
#[derive(Parser)]
#[command(version, group(ArgGroup::new("input").args(["file", "stdin", "paste"]).multiple(false)))]
#[command(group(ArgGroup::new("destination").args(["output", "copy", "serve_clipboard"]).required(true).multiple(false)))]
#[command(
    override_usage = "md2slack [OPTIONS] <--output <FILE>|--copy> [FILE]",
    after_help = "\n\nExamples:\n  md2slack notes.md -o notes.html\n  cat notes.md | md2slack --copy\n  md2slack --paste -o notes.html\n  md2slack --paste --copy\n\nHTML is a fragment for rich-text pasting, not Slack mrkdwn. Raw HTML in the input is preserved."
)]
struct Args {
    /// Read Markdown from a file (use - for stdin).
    file: Option<PathBuf>,
    /// Read Markdown from stdin (default when no input is specified).
    #[arg(long)]
    stdin: bool,
    /// Read Markdown text from the Wayland clipboard.
    #[arg(long)]
    paste: bool,
    /// Write HTML to a file.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    /// Copy HTML to the Wayland clipboard.
    #[arg(long)]
    copy: bool,
    /// Internal clipboard owner; launched by --copy.
    #[arg(long, hide = true)]
    serve_clipboard: bool,
}

fn run(args: Args) -> Result<()> {
    if args.serve_clipboard {
        return serve_clipboard();
    }
    let markdown = if args.paste {
        let (mut reader, _) = paste::get_contents(
            paste::ClipboardType::Regular,
            paste::Seat::Unspecified,
            paste::MimeType::Text,
        )
        .context("opening Wayland clipboard text")?;
        let mut text = String::new();
        reader
            .read_to_string(&mut text)
            .context("reading clipboard text as UTF-8")?;
        text
    } else if let Some(path) = args.file.filter(|path| path.as_os_str() != "-") {
        fs::read_to_string(&path)
            .with_context(|| format!("reading Markdown from {}", path.display()))?
    } else {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .context("reading Markdown from stdin")?;
        text
    };
    let html = md2slack::to_html(&markdown);
    if let Some(path) = args.output {
        fs::write(&path, html).with_context(|| format!("writing HTML to {}", path.display()))?;
    } else {
        copy_html(html)?;
    }
    Ok(())
}

fn copy_html(html: String) -> Result<()> {
    let mut child =
        Command::new(std::env::current_exe().context("locating clipboard owner binary")?)
            .arg("--serve-clipboard")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("starting clipboard owner")?;
    let mut input = child.stdin.take().expect("piped stdin");
    let written = input.write_all(html.as_bytes());
    drop(input);
    let mut response = String::new();
    BufReader::new(child.stdout.take().expect("piped stdout"))
        .read_line(&mut response)
        .context("waiting for clipboard owner readiness")?;
    if response.trim_end() != "ready" {
        let status = child.wait().context("waiting for failed clipboard owner")?;
        bail!("clipboard owner failed ({status}): {}", response.trim_end());
    }
    written.context("sending HTML to clipboard owner")?;
    // The owner remains alive until another application replaces the selection.
    Ok(())
}

fn serve_clipboard() -> Result<()> {
    let prepared = (|| {
        let mut html = Vec::new();
        io::stdin()
            .read_to_end(&mut html)
            .context("reading clipboard HTML")?;
        let mut options = copy::Options::new();
        options.foreground(true);
        options.omit_additional_text_mime_types(true);
        options
            .prepare_copy(
                copy::Source::Bytes(html.into_boxed_slice()),
                copy::MimeType::Specific("text/html".into()),
            )
            .context("preparing Wayland HTML clipboard")
    })();
    let mut ready = io::stdout().lock();
    let prepared = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            writeln!(ready, "{error:#}")?;
            ready.flush()?;
            return Err(error);
        }
    };
    writeln!(ready, "ready")?;
    ready.flush()?;
    drop(ready);
    prepared.serve().context("serving Wayland HTML clipboard")
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("md2slack: {error:#}");
            ExitCode::FAILURE
        }
    }
}
