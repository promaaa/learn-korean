# ADR-005: Remote images, metadata only

## Context
Learning items benefit from real photos. Storing image files would bloat the repository and the
user profile and raises licensing questions.

## Decision
The database stores `image_provider`, `image_url`, `source_url` and `attribution`. Images are
downloaded by Rust (`reqwest`) on demand, kept in memory and returned to the UI as bytes. Nothing
is written to disk by the app. Attribution is always displayed.

## Alternatives considered
- Bundling images: repository size, licensing.
- Hotlinking from the WebView: CORS, hotlink protection, CSP.

## Consequences
- Images require network access; the UI degrades gracefully without them.
- Providers are swappable behind an `ImageProvider` trait.
