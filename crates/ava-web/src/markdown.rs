//! A renderer for the markdown subset the task files and the analyses use:
//! headers, paragraphs, lists, tables, fenced code and inline code and
//! emphasis.

const HEADER_CLASSES: [&str; 3] = [
    "text-lg font-semibold text-neutral-100 mt-6 mb-2",
    "text-base font-semibold text-neutral-100 mt-5 mb-1",
    "font-semibold text-neutral-100 mt-4 mb-1",
];
const PARAGRAPH_CLASSES: &str = "my-2 max-w-prose leading-relaxed text-neutral-300";
const LIST_CLASSES: &str = "my-2 ml-5 max-w-prose leading-relaxed space-y-1 text-neutral-300";
const FENCE_CLASSES: &str = "font-mono text-xs text-neutral-300 bg-neutral-950 border border-neutral-800 \
     rounded-md p-3 my-3 overflow-x-auto";
const CODE_CLASSES: &str = "font-mono text-xs text-neutral-200 bg-neutral-800 rounded px-1 py-0.5";
const TABLE_CLASSES: &str = "my-3 text-left border-collapse";
const TABLE_HEADER_CLASSES: &str =
    "text-xs font-medium uppercase tracking-wider text-neutral-500 py-1.5 pr-4";
const TABLE_CELL_CLASSES: &str =
    "py-1.5 pr-4 border-t border-neutral-800 align-top text-neutral-300";

const FENCE: &str = "```";
const ROW: char = '|';
const SEPARATOR: [char; 2] = ['-', ':'];

/// What surrounds the line being rendered.
enum Block {
    Plain,
    Paragraph,
    List(&'static str),
    Fence,
    /// Whether the header row was written.
    Table(bool),
}

/// Render `markdown` into html, escaping everything the source wrote.
pub(crate) fn render(markdown: &str) -> String {
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
            match block {
                Block::List(open) if open == tag => {}
                open => {
                    close(&mut html, open);
                    html.push_str(&format!("<{tag} class=\"{LIST_CLASSES} {}\">", bullet(tag)));
                }
            }
            block = Block::List(tag);
            html.push_str(&format!("<li>{}</li>", inline(text)));
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

/// End whatever `block` opened.
fn close(html: &mut String, block: Block) {
    match block {
        Block::Plain => {}
        Block::Paragraph => html.push_str("</p>"),
        Block::List(tag) => html.push_str(&format!("</{tag}>")),
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
    if let Some(text) = line.strip_prefix("- ") {
        return Some(("ul", text));
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

/// Escape `text` and render its inline code spans and bold ranges.
fn inline(text: &str) -> String {
    let mut html = String::new();

    for (position, span) in escape(text).split('`').enumerate() {
        if position % 2 == 1 {
            html.push_str(&format!("<code class=\"{CODE_CLASSES}\">{span}</code>"));
            continue;
        }

        for (emphasis, run) in span.split("**").enumerate() {
            if emphasis % 2 == 1 {
                html.push_str(&format!("<strong>{run}</strong>"));
            } else {
                html.push_str(run);
            }
        }
    }

    html
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
