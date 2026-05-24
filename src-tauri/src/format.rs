enum Block {
    Heading(String),
    Paragraph(String),
    NumberedList(Vec<String>),
    BulletList(Vec<String>),
}

fn parse_numbered(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= bytes.len() {
        return None;
    }
    if bytes[i] == b'.' || bytes[i] == b')' {
        let rest = line[i + 1..].trim_start();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    None
}

fn parse_bullet(line: &str) -> Option<String> {
    const BULLETS: &[char] = &['•', '·', '‣', '◦', '▪', '▸', '–', '—'];
    let mut chars = line.chars();
    if let Some(first) = chars.next() {
        let rest_start = first.len_utf8();
        let rest = line[rest_start..].trim_start();
        if BULLETS.contains(&first) && !rest.is_empty() {
            return Some(rest.to_string());
        }
        if (first == '-' || first == '*') && line.len() > 1 && line.as_bytes()[1] == b' ' {
            let rest = line[2..].trim_start();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

fn is_heading(line: &str, is_first: bool, prev_empty: bool, next_empty: bool) -> bool {
    if line.len() > 80 || line.ends_with(',') || line.ends_with(';') {
        return false;
    }
    // First content line in the capture is almost always a title
    if is_first {
        return true;
    }
    // Short standalone line surrounded by blank lines
    if prev_empty && next_empty && line.len() <= 60 {
        return true;
    }
    // ALL CAPS short line (original heuristic, kept as fallback)
    let has_lower = line.chars().any(|c| c.is_lowercase());
    let has_alpha = line.chars().any(|c| c.is_alphabetic());
    !has_lower && has_alpha && line.len() <= 60
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn parse_blocks(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks: Vec<Block> = Vec::new();
    let mut first_content = true;
    let mut prev_empty = false;

    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();

        if line.is_empty() {
            prev_empty = true;
            continue;
        }

        let next_empty = lines.get(i + 1).map(|l| l.trim().is_empty()).unwrap_or(true);

        if let Some(item) = parse_numbered(line) {
            if let Some(Block::NumberedList(items)) = blocks.last_mut() {
                items.push(item);
            } else {
                blocks.push(Block::NumberedList(vec![item]));
            }
            first_content = false;
            prev_empty = false;
        } else if let Some(item) = parse_bullet(line) {
            if let Some(Block::BulletList(items)) = blocks.last_mut() {
                items.push(item);
            } else {
                blocks.push(Block::BulletList(vec![item]));
            }
            first_content = false;
            prev_empty = false;
        } else if is_heading(line, first_content, prev_empty, next_empty) {
            blocks.push(Block::Heading(line.to_string()));
            first_content = false;
            prev_empty = false;
        } else {
            // Continuation line: starts lowercase and directly follows a list item → join it
            let starts_lower = line.chars().next().map(|c| c.is_lowercase()).unwrap_or(false);
            let last_is_list = matches!(
                blocks.last(),
                Some(Block::BulletList(_)) | Some(Block::NumberedList(_))
            );

            if starts_lower && last_is_list && !prev_empty {
                match blocks.last_mut() {
                    Some(Block::BulletList(items)) | Some(Block::NumberedList(items)) => {
                        if let Some(last) = items.last_mut() {
                            last.push(' ');
                            last.push_str(line);
                        }
                    }
                    _ => {}
                }
            } else if let Some(Block::Paragraph(p)) = blocks.last_mut() {
                if !prev_empty {
                    p.push(' ');
                    p.push_str(line);
                } else {
                    blocks.push(Block::Paragraph(line.to_string()));
                }
            } else {
                blocks.push(Block::Paragraph(line.to_string()));
            }
            first_content = false;
            prev_empty = false;
        }
    }
    blocks
}

pub fn to_markdown(text: &str) -> String {
    let blocks = parse_blocks(text);
    let mut out: Vec<String> = Vec::new();
    for block in &blocks {
        match block {
            Block::Heading(h) => {
                if !out.is_empty() {
                    out.push(String::new());
                }
                out.push(format!("# {h}"));
                out.push(String::new());
            }
            Block::Paragraph(p) => {
                out.push(p.clone());
                out.push(String::new());
            }
            Block::NumberedList(items) => {
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{}. {item}", i + 1));
                }
                out.push(String::new());
            }
            Block::BulletList(items) => {
                for item in items {
                    out.push(format!("- {item}"));
                }
                out.push(String::new());
            }
        }
    }
    out.join("\n").trim().to_string()
}

pub fn to_html(text: &str) -> String {
    let blocks = parse_blocks(text);
    let mut body = String::new();
    for block in &blocks {
        match block {
            Block::Heading(h) => {
                body.push_str(&format!("<h1>{}</h1>", escape_html(h)));
            }
            Block::Paragraph(p) => {
                body.push_str(&format!("<p>{}</p>", escape_html(p)));
            }
            Block::NumberedList(items) => {
                body.push_str("<ol>");
                for item in items {
                    body.push_str(&format!("<li>{}</li>", escape_html(item)));
                }
                body.push_str("</ol>");
            }
            Block::BulletList(items) => {
                body.push_str("<ul>");
                for item in items {
                    body.push_str(&format!("<li>{}</li>", escape_html(item)));
                }
                body.push_str("</ul>");
            }
        }
    }
    format!("<html><body>{body}</body></html>")
}
