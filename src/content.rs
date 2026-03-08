use chrono::NaiveDate;
use pulldown_cmark::{html, Options, Parser};
use serde::Deserialize;
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
    fn from_file(path: &Path) -> Self {
        let content = fs::read_to_string(path).expect("failed to read post");
        let (fm, body) = split_frontmatter(&content);
        let frontmatter: Frontmatter =
            serde_yaml::from_str(&fm).expect("failed to parse frontmatter");

        let slug = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let word_count = body.split_whitespace().count();
        let reading_time = (word_count / 200).max(1);
        let content_html = render_markdown(&body);

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

fn render_markdown(raw: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(raw, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
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

    let mut posts: Vec<Post> = WalkDir::new(&posts_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| Post::from_file(e.path()))
        .filter(|p| !p.draft)
        .collect();

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    posts
}
