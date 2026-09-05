//! A renderer for the markdown subset the task files and the analyses use:
//! headers, paragraphs, block quotes, nested lists, tables, rules, fenced
//! code, and inline code, emphasis and links. Everything the source wrote is
//! escaped, so a rendered document cannot carry markup of its own.

const HEADER_CLASSES: [&str; 6] = [
    "text-lg font-semibold text-neutral-100 mt-6 mb-2",
    "text-base font-semibold text-neutral-100 mt-5 mb-1",
    "font-semibold text-neutral-100 mt-4 mb-1",
    "font-semibold text-neutral-200 mt-3 mb-1",
    "font-medium text-neutral-200 mt-3 mb-1",
    "font-medium text-neutral-300 mt-3 mb-1",
];
const PARAGRAPH_CLASSES: &str = "my-2 max-w-prose leading-relaxed text-neutral-300";
const QUOTE_CLASSES: &str = "my-2 max-w-prose border-l-2 border-neutral-700 pl-3 leading-relaxed \
     text-neutral-400";
const LIST_CLASSES: &str = "my-2 ml-5 max-w-prose leading-relaxed space-y-1 text-neutral-300";
const NESTED_LIST_CLASSES: &str = "mt-1 ml-5 space-y-1";
const RULE_CLASSES: &str = "my-4 border-neutral-800";
const FENCE_CLASSES: &str = "font-mono text-xs text-neutral-300 bg-neutral-950 border border-neutral-800 \
     rounded-md p-3 my-3 overflow-x-auto";
const CODE_CLASSES: &str = "font-mono text-xs text-neutral-200 bg-neutral-800 rounded px-1 py-0.5";
const LINK_CLASSES: &str =
    "text-indigo-300 hover:text-indigo-200 underline decoration-indigo-300/40";
const TABLE_CLASSES: &str = "my-3 text-left border-collapse";
const TABLE_HEADER_CLASSES: &str =
    "text-xs font-medium uppercase tracking-wider text-neutral-500 py-1.5 pr-4";
const TABLE_CELL_CLASSES: &str =
    "py-1.5 pr-4 border-t border-neutral-800 align-top text-neutral-300";

const FENCE: &str = "```";
const ROW: char = '|';
const QUOTE: &str = ">";
const SEPARATOR: [char; 2] = ['-', ':'];
const BULLETS: [&str; 3] = ["- ", "* ", "+ "];
const RULE_SIGNS: [char; 3] = ['-', '*', '_'];
const RULE_MINIMUM: usize = 3;
const BOLD: &str = "**";
const ITALIC: char = '*';
const CODE: char = '`';

/// The schemes a link may point at, besides a relative path or a fragment.
const LINK_SCHEMES: [&str; 2] = ["http://", "https://"];

/// What surrounds the line being rendered.
enum Block {
    Plain,
    Paragraph,
    Quote,
    /// The open lists, outermost first, each by the indent of its items and
    /// its tag. The innermost has an item open, closed with the list.
    List(Vec<(usize, &'static str)>),
    Fence,
    /// Whether the header row was written.
    Table(bool),
}

/// Render `markdown` into html, escaping everything the source wrote.
pub fn render(markdown: &str) -> String {
    let mut html = String::new();
    let mut block = Block::Plain;

    for line in markdown.lines() {
        if line.trim_start().starts_with(FENCE) {
            block = match block {
                Block::Fence => {
                    html.push_str("</pre>");
                    Block::Plain
                }
                open => {
                    close(&mut html, open);
                    html.push_str(&format!("<pre class=\"{FENCE_CLASSES}\">"));
                    Block::Fence
                }
            };
            continue;
        }

        if let Block::Fence = block {
            html.push_str(&escape(line));
            html.push('\n');
            continue;
        }

        let trimmed = line.trim();

        if trimmed.is_empty() {
            close(&mut html, block);
            block = Block::Plain;
            continue;
        }

        if let Some(text) = heading(trimmed) {
            close(&mut html, block);
            block = Block::Plain;
            html.push_str(&text);
            continue;
        }

        if rule(trimmed) {
            close(&mut html, block);
            block = Block::Plain;
            html.push_str(&format!("<hr class=\"{RULE_CLASSES}\">"));
            continue;
        }

        if let Some(text) = trimmed.strip_prefix(QUOTE) {
            match block {
                Block::Quote => html.push(' '),
                open => {
                    close(&mut html, open);
                    html.push_str(&format!("<blockquote class=\"{QUOTE_CLASSES}\">"));
                }
            }
            block = Block::Quote;
            html.push_str(&inline(text.trim()));
            continue;
        }

        if let Some(cells) = row(trimmed) {
            let headed = match block {
                Block::Table(headed) => headed,
                open => {
                    close(&mut html, open);
                    html.push_str(&format!("<table class=\"{TABLE_CLASSES}\">"));
                    false
                }
            };
            block = Block::Table(headed || !separator(&cells));
            if !separator(&cells) {
                let (tag, classes) = if headed {
                    ("td", TABLE_CELL_CLASSES)
                } else {
                    ("th", TABLE_HEADER_CLASSES)
                };
                html.push_str("<tr>");
                for cell in cells {
                    html.push_str(&format!(
                        "<{tag} class=\"{classes}\">{}</{tag}>",
                        inline(cell)
                    ));
                }
                html.push_str("</tr>");
            }
            continue;
        }

        if let Some((tag, text)) = item(trimmed) {
            let indent = line.len() - line.trim_start().len();
            let mut open = match block {
                Block::List(open) => open,
                other => {
                    close(&mut html, other);
                    Vec::new()
                }
            };
            list_item(&mut html, &mut open, indent, tag, text);
            block = Block::List(open);
            continue;
        }

        // A line following a list item at its indent or deeper continues that item.
        if let Block::List(open) = &block
            && line.len() - line.trim_start().len() > open.last().map_or(0, |(indent, _)| *indent)
        {
            html.push(' ');
            html.push_str(&inline(trimmed));
            continue;
        }

        match block {
            Block::Paragraph => html.push(' '),
            open => {
                close(&mut html, open);
                html.push_str(&format!("<p class=\"{PARAGRAPH_CLASSES}\">"));
            }
        }
        block = Block::Paragraph;
        html.push_str(&inline(trimmed));
    }

    close(&mut html, block);
    html
}

/// Write one list item at `indent`, opening a nested list inside the item
/// before it when it is deeper, closing lists when it is shallower.
fn list_item(
    html: &mut String,
    open: &mut Vec<(usize, &'static str)>,
    indent: usize,
    tag: &'static str,
    text: &str,
) {
    while let Some((deeper, closing)) = open.last().copied() {
        if deeper <= indent {
            break;
        }
        html.push_str(&format!("</li></{closing}>"));
        open.pop();
    }

    match open.last().copied() {
        Some((level, listed)) if level == indent && listed == tag => {
            html.push_str("</li>");
        }
        Some((level, listed)) if level == indent => {
            html.push_str(&format!("</li></{listed}>"));
            open.pop();
            html.push_str(&format!("<{tag} class=\"{LIST_CLASSES} {}\">", bullet(tag)));
            open.push((indent, tag));
        }
        Some(_) => {
            html.push_str(&format!(
                "<{tag} class=\"{NESTED_LIST_CLASSES} {}\">",
                bullet(tag)
            ));
            open.push((indent, tag));
        }
        None => {
            html.push_str(&format!("<{tag} class=\"{LIST_CLASSES} {}\">", bullet(tag)));
            open.push((indent, tag));
        }
    }

    html.push_str(&format!("<li>{}", inline(text)));
}

/// End whatever `block` opened.
fn close(html: &mut String, block: Block) {
    match block {
        Block::Plain => {}
        Block::Paragraph => html.push_str("</p>"),
        Block::Quote => html.push_str("</blockquote>"),
        Block::List(open) => {
            for (_, tag) in open.into_iter().rev() {
                html.push_str(&format!("</li></{tag}>"));
            }
        }
        Block::Fence => html.push_str("</pre>"),
        Block::Table(_) => html.push_str("</table>"),
    }
}

/// The cells of `line`, if it is a table row.
fn row(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix(ROW)?;
    let inner = inner.strip_suffix(ROW).unwrap_or(inner);
    Some(inner.split(ROW).map(str::trim).collect())
}

/// Whether `cells` are the row of dashes under a table header.
fn separator(cells: &[&str]) -> bool {
    cells
        .iter()
        .all(|cell| !cell.is_empty() && cell.chars().all(|sign| SEPARATOR.contains(&sign)))
}

/// Whether `line` is a horizontal rule: one sign repeated, three at least.
fn rule(line: &str) -> bool {
    let signs: Vec<char> = line.chars().filter(|sign| !sign.is_whitespace()).collect();

    signs.len() >= RULE_MINIMUM
        && RULE_SIGNS.contains(&signs[0])
        && signs.iter().all(|sign| *sign == signs[0])
}

/// The rendered heading, if `line` is one.
fn heading(line: &str) -> Option<String> {
    let level = line.chars().take_while(|&sign| sign == '#').count();
    if !(1..=HEADER_CLASSES.len()).contains(&level) || !line[level..].starts_with(' ') {
        return None;
    }

    Some(format!(
        "<h{level} class=\"{}\">{}</h{level}>",
        HEADER_CLASSES[level - 1],
        inline(line[level..].trim())
    ))
}

/// The list tag and the item text, if `line` is a list item.
fn item(line: &str) -> Option<(&'static str, &str)> {
    for bullet in BULLETS {
        if let Some(text) = line.strip_prefix(bullet) {
            return Some(("ul", text));
        }
    }

    let numbered = line.split_once(". ").filter(|(number, _)| {
        !number.is_empty() && number.chars().all(|digit| digit.is_ascii_digit())
    });

    numbered.map(|(_, text)| ("ol", text))
}

fn bullet(tag: &str) -> &'static str {
    if tag == "ul" {
        "list-disc"
    } else {
        "list-decimal"
    }
}

/// Escape `text` and render its inline code spans, emphasis and links.
fn inline(text: &str) -> String {
    let mut html = String::new();

    for (position, span) in escape(text).split(CODE).enumerate() {
        if position % 2 == 1 {
            html.push_str(&format!("<code class=\"{CODE_CLASSES}\">{span}</code>"));
            continue;
        }

        for (emphasis, run) in span.split(BOLD).enumerate() {
            if emphasis % 2 == 1 {
                html.push_str(&format!("<strong>{}</strong>", italic(run)));
            } else {
                html.push_str(&italic(run));
            }
        }
    }

    html
}

/// Render the italic ranges and the links of an escaped `text` holding no
/// code span and no bold marker.
///
/// A star toggles italics only when it starts a word or ends one, so a star
/// inside an expression stays a star.
fn italic(text: &str) -> String {
    let mut html = String::new();
    let mut open = false;
    let characters: Vec<char> = text.chars().collect();

    for (index, character) in characters.iter().enumerate() {
        if *character != ITALIC {
            html.push(*character);
            continue;
        }

        let before = index.checked_sub(1).map(|before| characters[before]);
        let after = characters.get(index + 1).copied();
        let opens = !open && after.is_some_and(|after| !after.is_whitespace());
        let closes = open && before.is_some_and(|before| !before.is_whitespace());

        if opens {
            html.push_str("<em>");
            open = true;
        } else if closes {
            html.push_str("</em>");
            open = false;
        } else {
            html.push(*character);
        }
    }

    if open {
        html.push_str("</em>");
    }

    links(&html)
}

/// Render every `[label](target)` of `text` as a link, when the target is a
/// web address, a path or a fragment.
fn links(text: &str) -> String {
    let mut html = String::new();
    let mut rest = text;

    while let Some(start) = rest.find('[') {
        let Some((label, after_label)) = rest[start + 1..].split_once("](") else {
            break;
        };
        let Some((target, after_target)) = after_label.split_once(')') else {
            break;
        };

        if label.is_empty() || !linkable(target) {
            html.push_str(&rest[..start + 1]);
            rest = &rest[start + 1..];
            continue;
        }

        html.push_str(&rest[..start]);
        html.push_str(&format!(
            "<a class=\"{LINK_CLASSES}\" href=\"{target}\">{label}</a>"
        ));
        rest = after_target;
    }

    html.push_str(rest);
    html
}

/// Whether `target` is somewhere a rendered link may point.
fn linkable(target: &str) -> bool {
    !target.is_empty()
        && !target.contains(char::is_whitespace)
        && (LINK_SCHEMES.iter().any(|scheme| target.starts_with(scheme))
            || target.starts_with('/')
            || target.starts_with('#')
            || !target.contains(':'))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn nested_lists_close_in_order() {
        let html = render("- one\n  - inner\n  - other\n- two\n\nafter");

        assert_eq!(html.matches("<ul").count(), 2);
        assert_eq!(html.matches("</ul>").count(), 2);
        assert_eq!(html.matches("<li>").count(), 4);
        assert_eq!(html.matches("</li>").count(), 4);
        assert!(html.contains("<li>one<ul"));
        assert!(
            html.ends_with(
                "<p class=\"my-2 max-w-prose leading-relaxed text-neutral-300\">after</p>"
            )
        );
    }

    #[test]
    fn a_continuation_line_joins_its_item() {
        let html = render("- one\n  continued\n- two");

        assert!(html.contains("<li>one continued</li>"));
        assert!(html.contains("<li>two</li>"));
    }

    #[test]
    fn quotes_rules_and_deep_headings_render() {
        let html = render("> quoted\n> more\n\n---\n\n#### fourth");

        assert!(html.contains("<blockquote"));
        assert!(html.contains("quoted more</blockquote>"));
        assert!(html.contains("<hr"));
        assert!(html.contains("<h4"));
    }

    #[test]
    fn inline_emphasis_and_links_render() {
        let html = render(
            "a *word* and **bold *both*** and `2 * 3` and [run](run.json) and [x](javascript:alert(1))",
        );

        assert!(html.contains("<em>word</em>"));
        assert!(html.contains("<strong>bold <em>both</em></strong>"));
        assert!(html.contains("<code class=\"font-mono text-xs text-neutral-200 bg-neutral-800 rounded px-1 py-0.5\">2 * 3</code>"));
        assert!(html.contains("<a class=\"text-indigo-300 hover:text-indigo-200 underline decoration-indigo-300/40\" href=\"run.json\">run</a>"));
        assert!(html.contains("[x](javascript:alert(1))"));
    }

    #[test]
    fn a_star_inside_an_expression_stays() {
        let html = render("2 * 3 * 4");

        assert!(!html.contains("<em>"));
        assert!(html.contains("2 * 3 * 4"));
    }
}
