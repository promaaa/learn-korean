# ADR-001: Use Tauri 2 with a Rust core and a Svelte 5 UI

## Context
The app must start instantly from a global shortcut on Linux (Omarchy/Hyprland), later run on
macOS and possibly iOS, and render Hangul typography, photos, audio, animations and an on-screen
keyboard. A TUI was considered for speed and keyboard navigation.

## Decision
Tauri 2 desktop shell, Rust for data/scheduling/providers, Svelte 5 + TypeScript + Vite for the UI,
pnpm for JavaScript packages.

## Alternatives considered
- **TUI (ratatui):** fastest start and native terminal feel, but images, audio, animations and a
  graphical Hangul keyboard are painful; no mobile path.
- **Electron:** mature, but bundles Chromium (size, memory, start time).
- **Native toolkits (GTK/SwiftUI):** one code base per platform.

## Consequences
- Uses the system WebView (WebKitGTK on Linux, WKWebView on macOS/iOS).
- Keyboard-only UX has to be designed deliberately; the web layer makes it easy.
- Domain logic must stay in Rust crates so that other front ends remain possible.
