use crate::config::SiteConfig;
use crate::content::Post;
use std::path::Path;
use tera::{Context, Tera};

pub struct Renderer {
    tera: Tera,
}

impl Renderer {
    pub fn new(templates_dir: &Path) -> Self {
        let glob = templates_dir.join("**").join("*.html");
        let tera = Tera::new(glob.to_str().unwrap()).expect("failed to load templates");
        Self { tera }
    }

    pub fn render_post(
        &self,
        post: &Post,
        prev: Option<&Post>,
        next: Option<&Post>,
        config: &SiteConfig,
    ) -> String {
        let mut ctx = Context::new();
        ctx.insert("title", &post.title);
        ctx.insert("description", &post.description);
        ctx.insert("date", &post.date.to_string());
        ctx.insert("tags", &post.tags);
        ctx.insert("content", &post.content_html);
        ctx.insert("reading_time", &post.reading_time);
        ctx.insert("slug", &post.slug);
        ctx.insert("site", config);
        if let Some(p) = prev {
            ctx.insert("prev_post", p);
        }
        if let Some(n) = next {
            ctx.insert("next_post", n);
        }
        self.tera
            .render("post.html", &ctx)
            .expect("failed to render post")
    }

    pub fn render_index(&self, posts: &[Post], config: &SiteConfig) -> String {
        let mut ctx = Context::new();
        ctx.insert("posts", posts);
        ctx.insert("site", config);
        self.tera
            .render("index.html", &ctx)
            .expect("failed to render index")
    }

    pub fn render_tags_index(
        &self,
        tags: &[(String, usize)],
        config: &SiteConfig,
    ) -> String {
        let mut ctx = Context::new();
        ctx.insert("tags", tags);
        ctx.insert("site", config);
        self.tera
            .render("tags.html", &ctx)
            .expect("failed to render tags index")
    }

    pub fn render_404(&self, config: &SiteConfig) -> String {
        let mut ctx = Context::new();
        ctx.insert("site", config);
        self.tera
            .render("404.html", &ctx)
            .expect("failed to render 404")
    }

    pub fn render_tag(&self, tag: &str, posts: &[&Post], config: &SiteConfig) -> String {
        let mut ctx = Context::new();
        ctx.insert("tag", tag);
        ctx.insert("posts", &posts);
        ctx.insert("site", config);
        self.tera
            .render("tag.html", &ctx)
            .expect("failed to render tag page")
    }
}
