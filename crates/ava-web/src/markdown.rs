//! A renderer for the markdown subset the task files use: headers,
//! paragraphs, lists, fenced code and inline code and emphasis.

const HEADER_CLASSES: [&str; 3] = [
    "text-lg font-semibold mt-6 mb-2",
    "text-base font-semibold mt-5 mb-1",
    "font-semibold mt-4 mb-1",
];
const PARAGRAPH_CLASSES: &str = "my-2 max-w-prose leading-relaxed";
const LIST_CLASSES: &str = "my-2 ml-5 max-w-prose leading-relaxed space-y-1";
const FENCE_CLASSES: &str = "font-mono text-[13px] bg-neutral-50 -mx-3 p-3 my-3 overflow-x-auto";
const CODE_CLASSES: &str = "font-mono text-[13px] bg-neutral-100 px-1";

const FENCE: &str = "```";

/// What surrounds the line being rendered.
enum Block {
    Plain,
    Paragraph,
    List(&'static str),
    Fence,
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
    }
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
