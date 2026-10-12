use pulldown_cmark::{Event, Options, Parser, Tag, html};
use std::fmt::Write;

/// Convert GitHub-flavored Markdown to an HTML fragment to paste as rich-text into Slack.
///
/// In the conversion, the following rules esure Slack renders lists correctly:
///
/// 1. Nested lists become sibling list
/// 2. Ordered runs preserve their starting number when resumed after nested lists.
/// 3. Raw HTML is preserved.
pub fn to_html(markdown: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    let events: Vec<_> = Parser::new_ext(markdown, options).collect();
    // Index matching ends once, so walking nested blocks stays linear.
    let mut ends = vec![0; events.len()];
    let mut stack = Vec::new();
    for (index, event) in events.iter().enumerate() {
        match event {
            Event::Start(_) => stack.push(index),
            Event::End(_) => ends[stack.pop().expect("balanced parser events")] = index,
            _ => {}
        }
    }
    let renderer = Renderer { events, ends };
    let mut output = String::new();
    let mut blocks = Vec::new();
    renderer.blocks(0, renderer.events.len(), &mut blocks);
    html::push_html(&mut output, blocks.into_iter());
    output
}

struct Renderer<'a> {
    events: Vec<Event<'a>>,
    ends: Vec<usize>,
}

impl<'a> Renderer<'a> {
    /// Replace lists in arbitrary blocks & quotes, with raw HTML.
    fn blocks(&self, start: usize, end: usize, output: &mut Vec<Event<'a>>) {
        let mut index = start;
        while index < end {
            if matches!(self.events[index], Event::Start(Tag::List(_))) {
                let mut list = String::new();
                self.list(index, 0, &mut list);
                output.push(Event::Html(list.into()));
                index = self.ends[index] + 1;
            } else {
                output.push(self.events[index].clone());
                index += 1;
            }
        }
    }

    fn list(&self, start: usize, indent: usize, output: &mut String) {
        let Event::Start(Tag::List(number)) = self.events[start] else {
            unreachable!("list opener");
        };
        let (tag, kind) = if number.is_some() {
            ("ol", "ordered-list")
        } else {
            ("ul", "unordered-list")
        };
        let mut number = number;
        let mut open = false;
        let mut item = start + 1;
        while item < self.ends[start] {
            debug_assert!(matches!(self.events[item], Event::Start(Tag::Item)));
            let end = self.ends[item];
            let mut text = Vec::new();
            let mut children = Vec::new();
            let mut block = item + 1;
            // Only direct child lists are deferred; lists inside other blocks
            // remain part of that block, as in the original list filter.
            while block < end {
                let next = if matches!(self.events[block], Event::Start(_)) {
                    self.ends[block] + 1
                } else {
                    block + 1
                };
                if matches!(self.events[block], Event::Start(Tag::List(_))) {
                    children.push(block);
                } else {
                    self.blocks(block, next, &mut text);
                }
                block = next;
            }
            if !open {
                write!(
                    output,
                    "<{tag} data-stringify-type=\"{kind}\" data-indent=\"{indent}\""
                )
                .expect("writing to String");
                if let Some(number) = number {
                    write!(output, " start=\"{number}\"").expect("writing to String");
                }
                output.push('>');
                open = true;
            }
            let mut rendered = String::new();
            html::push_html(&mut rendered, text.into_iter());
            write!(output, "<li data-stringify-indent=\"{indent}\">").expect("writing to String");
            append_item(output, &rendered);
            output.push_str("</li>");
            number = number.map(|number| number + 1);
            if !children.is_empty() {
                write!(output, "</{tag}>").expect("writing to String");
                open = false;
                for child in children {
                    self.list(child, indent + 1, output);
                }
            }
            item = end + 1;
        }
        if open {
            write!(output, "</{tag}>").expect("writing to String");
        }
    }
}

/// Match the filter's paragraph stripping and whitespace normalization.
///
/// Does not allocate a second copy of the rendered item.
fn append_item(output: &mut String, mut html: &str) {
    let mut pending_space = false;
    let mut has_text = false;
    while !html.is_empty() {
        if let Some(rest) = html
            .strip_prefix("<p>")
            .or_else(|| html.strip_prefix("</p>"))
        {
            html = rest;
            continue;
        }
        let ch = html.chars().next().expect("nonempty HTML");
        html = &html[ch.len_utf8()..];
        if ch.is_ascii_whitespace() {
            pending_space = has_text;
        } else {
            if pending_space {
                output.push(' ');
                pending_space = false;
            }
            output.push(ch);
            has_text = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::to_html;

    #[test]
    fn nested_runs_resume_numbering_and_track_depth() {
        assert_eq!(
            to_html("3. parent\n   - child\n     1. deep\n4. resumed\n"),
            concat!(
                "<ol data-stringify-type=\"ordered-list\" data-indent=\"0\" start=\"3\">",
                "<li data-stringify-indent=\"0\">parent</li></ol>",
                "<ul data-stringify-type=\"unordered-list\" data-indent=\"1\">",
                "<li data-stringify-indent=\"1\">child</li></ul>",
                "<ol data-stringify-type=\"ordered-list\" data-indent=\"2\" start=\"1\">",
                "<li data-stringify-indent=\"2\">deep</li></ol>",
                "<ol data-stringify-type=\"ordered-list\" data-indent=\"0\" start=\"4\">",
                "<li data-stringify-indent=\"0\">resumed</li></ol>"
            )
        );
    }

    #[test]
    fn item_paragraphs_precede_deferred_sublists() {
        assert_eq!(
            to_html("- first\n\n  second\n\n  - child\n\n  final\n\n- next\n"),
            concat!(
                "<ul data-stringify-type=\"unordered-list\" data-indent=\"0\">",
                "<li data-stringify-indent=\"0\">first second final</li></ul>",
                "<ul data-stringify-type=\"unordered-list\" data-indent=\"1\">",
                "<li data-stringify-indent=\"1\">child</li></ul>",
                "<ul data-stringify-type=\"unordered-list\" data-indent=\"0\">",
                "<li data-stringify-indent=\"0\">next</li></ul>"
            )
        );
    }

    #[test]
    fn inline_markup_and_escaping_survive_flattening() {
        assert_eq!(
            to_html("- **bold** & [link](https://example.com) `a<b` ~~old~~\n"),
            concat!(
                "<ul data-stringify-type=\"unordered-list\" data-indent=\"0\">",
                "<li data-stringify-indent=\"0\"><strong>bold</strong> &amp; ",
                "<a href=\"https://example.com\">link</a> <code>a&lt;b</code> ",
                "<del>old</del></li></ul>"
            )
        );
    }
}
