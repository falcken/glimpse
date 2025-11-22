# Glimpse

Glimpse is a modern, fast Markdown renderer and previewer built with Tauri and Vite. It provides real-time rendering of Markdown content with support for LaTeX mathematics, synchronized scrolling, and editor integration.

## Features

- **Real-time Rendering**: Instantly renders Markdown as you type.
- **LaTeX Support**: Full support for mathematical equations using KaTeX (inline and block math).
- **Synchronized Scrolling**: Automatically scrolls to the current cursor position in your editor.
- **Editor Integration**: CMD+Click on any line in the preview to jump to that line in your editor.
- **Customizable**: Settings for LaTeX preambles and other preferences.
- **Minimalist UI**: Clean interface with a custom title bar.

## Tech Stack

Tauri (Rust + TypeScript), Markdown-it, KaTeX, Vanilla CSS.

## Development

### Prerequisites

- Node.js (pnpm recommended)
- Rust (cargo)

### Setup

1. Clone the repository.
2. Install dependencies:
   ```bash
   pnpm install
   ```

### Running

To start the development server:

```bash
pnpm tauri dev
```

### Building

To build the application for production:

```bash
pnpm tauri build
```

## Usage

Glimpse is designed to work alongside your favorite text editor. It listens for markdown updates and renders them immediately.

- **Math**: Use `$` for inline math and `$$` for block math.
- **Navigation**: CMD+Click (or CTRL+Click) on a line to open it in your editor.
