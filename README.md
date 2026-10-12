# Markdown 2 Slack `md2slack`

## Project description

This is a simple CLI to convert markdown into HTML that can be pasted into
Slack. This replaces a `lua` filter I previously used [^b996d68].

This is for anyone who wants to write a reply in vim and just paste it in.

## Project dependencies

You'll need `rust` to compile it.

> [!NOTE]
> Clipboard operations use the built-in `wl-clipboard-rs` crate and require a
> Wayland session with data-control support.

`--copy` starts a background instance of the same binary to serve `text/html`
after the command exits. It stops when the clipboard selection is replaced.

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
         --paste          Read Markdown text from the Wayland clipboard
     -o, --output <FILE>  Write HTML to a file
         --copy           Copy HTML to the Wayland clipboard
     -h, --help           Print help
     -V, --version        Print version

   Clipboard access is built in and requires a Wayland session with data-control support.

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

## Footnotes

[^b996d68]: The following `lua` filter can also be used:

    ```sh
    wl-paste | pandoc -f gfm -t html --lua-filter slack-lists.lua | wl-copy -t text/html
    ```

    with the following lua filter:

    ```lua
    -- Flatten nested lists into Slack-style indented list runs
    local function render(blocks)
    	local html = pandoc.write(pandoc.Pandoc(blocks), "html")
    	html = html:gsub("</?p>", ""):gsub("%s+", " "):gsub("^ ", ""):gsub(" $", "")
    	return html
    end

    local function walk(list, indent, chunks)
    	local tag = list.t == "OrderedList" and "ol" or "ul"
    	local n = list.t == "OrderedList" and list.listAttributes.start or 1
    	local cur = nil
    	for _, item in ipairs(list.content) do
    		local text, subs = {}, {}
    		for _, b in ipairs(item) do
    			if b.t == "BulletList" or b.t == "OrderedList" then
    				table.insert(subs, b)
    			else
    				table.insert(text, b)
    			end
    		end
    		if not cur then
    			cur = { tag = tag, indent = indent, start = n, items = {} }
    			table.insert(chunks, cur)
    		end
    		table.insert(cur.items, render(text))
    		n = n + 1
    		for _, s in ipairs(subs) do
    			walk(s, indent + 1, chunks)
    			cur = nil -- resume parent level in a new run
    		end
    	end
    end

    local function flatten(list)
    	local chunks = {}
    	walk(list, 0, chunks)
    	local out = {}
    	for _, c in ipairs(chunks) do
    		local kind = c.tag == "ol" and "ordered-list" or "unordered-list"
    		local start = c.tag == "ol" and (' start="' .. c.start .. '"') or ""
    		table.insert(
    			out,
    			string.format('<%s data-stringify-type="%s" data-indent="%d"%s>', c.tag, kind, c.indent, start)
    		)
    		for _, li in ipairs(c.items) do
    			table.insert(out, string.format('<li data-stringify-indent="%d">%s</li>', c.indent, li))
    		end
    		table.insert(out, "</" .. c.tag .. ">")
    	end
    	return pandoc.RawBlock("html", table.concat(out))
    end

    return {
    	traverse = "topdown",
    	BulletList = function(l)
    		return flatten(l), false
    	end,
    	OrderedList = function(l)
    		return flatten(l), false
    	end,
    }
    ```
