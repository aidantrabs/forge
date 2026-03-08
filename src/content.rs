use crate::highlight::Highlighter;
use chrono::NaiveDate;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Deserialize)]
struct Frontmatter {
    title: String,
    description: String,
    date: NaiveDate,
    tags: Vec<String>,
    #[serde(default)]
    draft: bool,
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

        Post {
            title: frontmatter.title,
            description: frontmatter.description,
            slug,
            date: frontmatter.date,
            tags: frontmatter.tags,
            draft: frontmatter.draft,
            content_html,
            reading_time,
        }
    }
}

fn render_markdown(raw: &str, highlighter: &Highlighter) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(raw, options);
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
    html_output
}

fn sanitize_html(html: &str) -> String {
    let mut builder = ammonia::Builder::default();
    builder
        .add_generic_attributes(&["style"])
        .add_tags(&["pre", "code", "span", "div"]);
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

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    posts
}
