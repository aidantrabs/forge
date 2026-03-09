# forge

my static site generator for developer blogs, built with rust.

## stack

- **generator** - rust (pulldown-cmark, tera, syntect, ammonia)
- **frontend** - vite 6, vanilla typescript, tailwind css v4
- **hosting** - cloudflare pages

## usage

```bash
npm install
npm run build && cargo run -- build
```

```bash
cargo run -- new "post title"
cargo run -- clean
```

## structure

```
src/              # rust generator
frontend/         # vite + typescript + css
templates/        # tera html templates
content/posts/    # markdown posts
static/           # built css/js + assets
forge.toml        # site config
output/           # generated site
```

## content

posts live in `content/posts/` as markdown with yaml frontmatter:

```markdown
---
title: "my post"
description: "a short description"
date: 2026-03-08
tags: ["rust", "web"]
draft: false
---

post content here.
```
