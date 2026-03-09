---
title: "building forge"
description: "how i built a rust static site generator from scratch"
date: 2026-03-08
tags: ["rust", "devtools", "web"]
draft: false
---

every developer eventually builds their own blog engine. this is mine.

## the stack

forge is a two-part system:

- a **rust cli** that parses markdown, applies templates, and outputs a static site
- a **minimal frontend** built with vite, vanilla typescript, and tailwind v4

the goal was simple: fast builds, tiny output, zero runtime complexity.

## why rust
> i'm a wannabe rustacean

rust gives us:

1. fast compilation of markdown to html
2. zero-cost abstractions for template rendering
3. a single binary with no runtime dependencies

```rust
let posts = load_posts(Path::new("content"));
let renderer = Renderer::new(Path::new("templates"));

for post in &posts {
    let html = renderer.render_post(post, &config);
    fs::write(post_dir.join("index.html"), html)?;
}
```

## the frontend

the entire javascript runtime is under 1kb. it handles:

- dark mode toggle via a pull-string ui element
- scroll reveal animations with `IntersectionObserver`
- font loading with fout prevention

> the best javascript is the javascript you don't ship.

## what's next

- wasm-powered client-side search
- image optimization pipeline
- incremental builds
