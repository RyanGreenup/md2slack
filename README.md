# Markdown 2 Slack `md2slack`

## Project description

This is a simple CLI to convert markdown into HTML that can be pasted into
Slack.

This is for anyone who wants to write a reply in vim and just paste it in.

## Project dependencies

You'll need `rust` to compile it.

> [!NOTE]
> Currently, clipboard operations require a Wayland session and `wl-clipboard`
> (`wl-paste` and `wl-copy`).

## Instructions for using Markdown 2 Slack

1. Install it
   ```sh
   cd ~/Downloads/
   git clone https://github.com/ryangreenup/md2slack.git
   cd md2slack
   cargo install --path cli
   ```
2. Run it
   ```sh
   md2slack --help
   ```
   ```
   Convert GFM Markdown to HTML that can be pasted into Slack

   Usage: md2slack [OPTIONS] <--output <FILE>|--copy> [FILE]

   Arguments:
     [FILE]  Read Markdown from a file (use - for stdin)

   Options:
         --stdin          Read Markdown from stdin (default when no input is specified)
         --paste          Read Markdown text from the Wayland clipboard using wl-paste
     -o, --output <FILE>  Write HTML to a file
         --copy           Copy HTML to the Wayland clipboard using wl-copy
     -h, --help           Print help
     -V, --version        Print version

   Markdown conversion is built in. Clipboard operations require a Wayland session and wl-clipboard.

   Examples:
     md2slack notes.md -o notes.html
     cat notes.md | md2slack --copy
     md2slack --paste -o notes.html
     md2slack --paste --copy

   HTML is a fragment for rich-text pasting, not Slack mrkdwn. Raw HTML in the input is preserved.
   ```

## Contributing guidelines

PR's welcome.

## How to get help

Issues welcome.

## Terms of use

Markdown 2 Slack is licensed under MIT.
