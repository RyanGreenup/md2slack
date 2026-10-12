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
