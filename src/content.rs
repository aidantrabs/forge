use crate::highlight::Highlighter;
use chrono::NaiveDate;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use walkdir::WalkDir;

#[derive(Deserialize)]
struct Frontmatter {
    title: String,
    description: String,
    date: NaiveDate,
    tags: Vec<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    weight: Option<i32>,
}

#[derive(Serialize)]
pub struct Post {
    pub title: String,
    pub description: String,
    pub slug: String,
    pub date: NaiveDate,
    pub tags: Vec<String>,
    pub draft: bool,
    pub content_html: String,
    pub reading_time: usize,
    #[serde(skip)]
    pub weight: i32,
    #[serde(skip)]
    pub modified: u64,
}

impl Post {
    fn from_file(path: &Path, highlighter: &Highlighter) -> Self {
        let content = fs::read_to_string(path).expect("failed to read post");
        let (fm, body) = split_frontmatter(&content);
        let frontmatter: Frontmatter =
            serde_yaml::from_str(&fm).expect("failed to parse frontmatter");

        let slug = path.file_stem().unwrap().to_string_lossy().into_owned();

        let word_count = body.split_whitespace().count();
        let reading_time = (word_count / 200).max(1);
        let raw_html = render_markdown(&body, highlighter);
        let content_html = sanitize_html(&raw_html);

        let modified = fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Post {
            title: frontmatter.title,
            description: frontmatter.description,
            slug,
            date: frontmatter.date,
            tags: frontmatter.tags,
            draft: frontmatter.draft,
            content_html,
            reading_time,
            weight: frontmatter.weight.unwrap_or(0),
            modified,
        }
    }
}

fn count_backticks(chars: &mut std::iter::Peekable<std::str::Chars>, result: &mut String) -> usize {
    let mut count = 0;
    while chars.peek() == Some(&'`') {
        result.push(chars.next().unwrap());
        count += 1;
    }
    count
}

fn protect_math(raw: &str) -> (String, Vec<String>) {
    let mut result = String::new();
    let mut math_blocks: Vec<String> = Vec::new();
    let mut chars = raw.chars().peekable();
    let mut at_line_start = true;

    while let Some(&ch) = chars.peek() {
        if ch == '\n' {
            result.push(chars.next().unwrap());
            at_line_start = true;
        } else if ch == '`' {
            let backtick_count = count_backticks(&mut chars, &mut result);

            if backtick_count >= 3 && at_line_start {
                while let Some(&c) = chars.peek() {
                    if c == '\n' {
                        break;
                    }
                    result.push(chars.next().unwrap());
                }
                loop {
                    match chars.peek() {
                        None => break,
                        Some(&'\n') => {
                            result.push(chars.next().unwrap());
                        }
                        Some(&'`') => {
                            let count = count_backticks(&mut chars, &mut result);
                            if count >= backtick_count {
                                while let Some(&c) = chars.peek() {
                                    if c == '\n' {
                                        break;
                                    }
                                    result.push(chars.next().unwrap());
                                }
                                break;
                            }
                            while let Some(&c) = chars.peek() {
                                if c == '\n' {
                                    break;
                                }
                                result.push(chars.next().unwrap());
                            }
                        }
                        Some(_) => {
                            while let Some(&c) = chars.peek() {
                                if c == '\n' {
                                    break;
                                }
                                result.push(chars.next().unwrap());
                            }
                        }
                    }
                }
            } else {
                at_line_start = false;
                loop {
                    match chars.peek() {
                        None => break,
                        Some(&'`') => {
                            let count = count_backticks(&mut chars, &mut result);
                            if count == backtick_count {
                                break;
                            }
                        }
                        Some(_) => {
                            result.push(chars.next().unwrap());
                        }
                    }
                }
            }
        } else if ch == '$' {
            at_line_start = false;
            chars.next();
            if chars.peek() == Some(&'$') {
                chars.next();
                let mut math = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '$' {
                        chars.next();
                        if chars.peek() == Some(&'$') {
                            chars.next();
                            break;
                        }
                        math.push('$');
                    } else {
                        math.push(chars.next().unwrap());
                    }
                }
                let idx = math_blocks.len();
                math_blocks.push(format!("$${math}$$"));
                result.push_str(&format!("\x00MATH{idx}\x00"));
            } else {
                let mut math = String::new();
                let mut found_end = false;
                while let Some(&c) = chars.peek() {
                    if c == '$' {
                        chars.next();
                        found_end = true;
                        break;
                    }
                    math.push(chars.next().unwrap());
                }
                if found_end && !math.is_empty() {
                    let idx = math_blocks.len();
                    math_blocks.push(format!("${math}$"));
                    result.push_str(&format!("\x00MATH{idx}\x00"));
                } else {
                    result.push('$');
                    result.push_str(&math);
                }
            }
        } else {
            if ch != ' ' && ch != '\t' {
                at_line_start = false;
            }
            result.push(chars.next().unwrap());
        }
    }

    (result, math_blocks)
}

fn restore_math(html: &str, math_blocks: &[String]) -> String {
    let mut result = html.to_string();
    for (i, block) in math_blocks.iter().enumerate() {
        let placeholder = format!("\x00MATH{i}\x00");
        result = result.replace(&placeholder, block);
    }
    result
}

fn render_markdown(raw: &str, highlighter: &Highlighter) -> String {
    let (protected, math_blocks) = protect_math(raw);

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(&protected, options);
    let mut html_output = String::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buf = String::new();

    let events: Vec<Event> = parser
        .flat_map(|event| match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                in_code_block = true;
                code_lang = lang.to_string();
                code_buf.clear();
                vec![]
            }
            Event::Text(text) if in_code_block => {
                code_buf.push_str(&text);
                vec![]
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                let highlighted = highlighter.highlight(&code_buf, &code_lang);
                vec![Event::Html(highlighted.into())]
            }
            other => vec![other],
        })
        .collect();

    pulldown_cmark::html::push_html(&mut html_output, events.into_iter());
    restore_math(&html_output, &math_blocks)
}

fn sanitize_html(html: &str) -> String {
    let mut builder = ammonia::Builder::default();
    builder
        .add_generic_attributes(&["style"])
        .add_tags(&[
            "pre", "code", "span", "div", "table", "thead", "tbody", "tr", "th", "td",
        ]);
    builder.clean(html).to_string()
}

fn split_frontmatter(content: &str) -> (String, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (String::new(), content.to_string());
    }

    let after_opening = &trimmed[3..];
    let end = after_opening
        .find("\n---")
        .expect("missing closing frontmatter delimiter");

    let fm = after_opening[..end].trim().to_string();
    let body = after_opening[end + 4..].trim().to_string();
    (fm, body)
}

pub fn load_posts(content_dir: &Path) -> Vec<Post> {
    let posts_dir = content_dir.join("posts");
    let highlighter = Highlighter::new();

    let mut posts: Vec<Post> = WalkDir::new(&posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| Post::from_file(e.path(), &highlighter))
        .filter(|p| !p.draft)
        .collect();

    posts.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then(b.weight.cmp(&a.weight))
            .then(b.modified.cmp(&a.modified))
    });
    posts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_title_in_code() {
        let input = "<p>the <code>&lt;title&gt;</code> showed</p>";
        let result = sanitize_html(input);
        println!("Input:  {}", input);
        println!("Output: {}", result);
        assert!(result.contains("&lt;title&gt;"), "title entity should survive sanitization");
    }

    #[test]
    fn test_protect_math_inline_code_with_backticks() {
        let input = "switching to `<title>{`req-${reqId}`}</title>` works";
        let (result, blocks) = protect_math(input);
        println!("Input:  {}", input);
        println!("Result: {}", result);
        println!("Blocks: {:?}", blocks);
        assert!(blocks.is_empty(), "no math blocks should be created");
        assert_eq!(input, result, "content should pass through unchanged");
    }

    #[test]
    fn test_protect_math_code_fence() {
        let input = "before\n```tsx\n<title>{`req-${reqId}`}</title>\n```\nafter";
        let (result, blocks) = protect_math(input);
        println!("Input:  {}", input);
        println!("Result: {}", result);
        println!("Blocks: {:?}", blocks);
        assert!(blocks.is_empty(), "no math blocks should be created inside code fence");
        assert_eq!(input, result, "content should pass through unchanged");
    }

    #[test]
    fn test_full_pipeline_code_fence_with_template_literals() {
        let highlighter = Highlighter::new();
        let input = "text before\n\n```tsx\nexport default function Page({ id }: Props) {\n  return <title>{`req-${id}`}</title>;\n}\n```\n\ntext after";
        let raw_html = render_markdown(input, &highlighter);
        let html = sanitize_html(&raw_html);
        println!("Output: {}", html);
        assert!(html.contains("req-"), "template literal content should be present");
        assert!(html.contains("text before"), "content before fence preserved");
        assert!(html.contains("text after"), "content after fence preserved");
    }

    #[test]
    fn test_full_pipeline_inline_code_with_title() {
        let highlighter = Highlighter::new();
        let input = "the `<title>` showed `req-13`";
        let raw_html = render_markdown(input, &highlighter);
        let html = sanitize_html(&raw_html);
        println!("Output: {}", html);
        assert!(html.contains("&lt;title&gt;"), "title tag should be escaped in inline code");
    }

    #[test]
    fn test_protect_math_dollar_after_backtick_fence() {
        let input = "before\n```\nlet x = $foo\n```\n$math$ after";
        let (result, blocks) = protect_math(input);
        println!("Result: {}", result);
        println!("Blocks: {:?}", blocks);
        assert_eq!(blocks.len(), 1, "only the real math outside fence should be captured");
        assert!(blocks[0].contains("math"), "the math block should be $math$");
    }
}
