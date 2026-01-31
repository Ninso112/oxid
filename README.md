# Oxid ⚡

[![Built in Rust](https://img.shields.io/badge/built%20in-Rust-orange.svg)](https://www.rust-lang.org)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

## About

**Oxid** is a terminal-based Markdown editor with Vim-like keybindings, written in Rust using [Ratatui](https://ratatui.rs). It provides a fast, keyboard-driven workflow for managing and editing notes without leaving the terminal.

---

## Features

### Vim Motions

- **Normal** and **Insert** modes – standard Vim-style editing.
- `h` / `j` / `k` / `l` for navigation, `i` / `a` to enter Insert mode, `Esc` to return to Normal mode.
- `u` for undo and familiar movement keys.

### Workspace

- **File explorer** – browse your notes directory in a side panel.
- **Preview pane** – select files to see their content rendered as Markdown in real time.
- Quick navigation between list and editor with `q` (back) and `Enter` (open).

### Advanced Search

- **Fuzzy search** triggered by `/` – search across filenames and file contents.
- Prioritizes filename matches, then content matches.
- Live highlighting of matched characters.
- Filter by tags with `#tag` (e.g. `#work`).

### Layout

- **Tabs** – multiple files open at once (`gt` / `gT` to switch).
- **Split view** – view two files side-by-side (`gs` to toggle, `Tab` to switch pane focus).
- **Zen mode** (`F11`) – hide list and preview for distraction-free editing.

### Writing Aids

- **Typewriter mode** – keeps the cursor vertically centered in the viewport (configurable).
- **Spellcheck** – multi-language spell checking with configurable dictionaries (`en`, `de`, etc.).

### Syntax Highlighting

- Markdown syntax in the editor: headers (`#`), lists (`-`), and checkboxes (`- [ ]` / `- [x]`).
- **Dynamic checkboxes** – `[ ]` (unchecked) and `[x]` (checked) rendered with distinct styles; checked items use strikethrough in the preview pane.

### Export

- **PDF export** via Pandoc – press `Ctrl+e` to export the current file to PDF.
- Requires [Pandoc](https://pandoc.org) to be installed.

### Additional

- **Wiki-links** – `[[Filename]]` to navigate; creates the file if it does not exist.
- **Command palette** (`Ctrl+p`) – Rename, Delete, Insert Date, Toggle Zen/Split, Git Push, Export PDF.
- **Templates** – Empty, Daily Note, or Meeting when creating new files.
- **Git status** – footer shows "Git: Clean" or "Git: Dirty" when the notes directory is a repository.
- **Frontmatter / tags** – YAML tags in Markdown for filtering in search.

---

## Installation

### Prerequisites

- **Rust** and **Cargo** (stable toolchain, 1.70+ recommended)
- **Pandoc** (optional, for PDF export)

```bash
# Check Rust
rustc --version
cargo --version

# Install Pandoc (optional, for PDF export)
# Debian/Ubuntu: sudo apt install pandoc
# Arch: sudo pacman -S pandoc
```

### Build

```bash
git clone https://github.com/Ninso112/oxid.git
cd oxid
cargo build --release
```

The binary will be at `target/release/oxid`.

To install system-wide:

```bash
cargo install --path .
```

---

## Configuration

Oxid follows the Linux XDG Base Directory standard. Configuration is created automatically at first run in `~/.config/oxid/`.

### config.toml

```toml
# Notes directory
notes_directory = "~/Documents/Notes"

[editor]
typewriter_mode = false      # Keep cursor vertically centered
enable_spellcheck = false    # Multi-language spell checking
spellcheck_languages = ["en"]

[keys]
search = "/"                 # Trigger fuzzy search
pdf_export = "ctrl+e"        # Export current file to PDF
```

### theme.toml

Colors and styles are fully configurable via `theme.toml`. Hex (`#RRGGBB`) or named colors are supported for all UI elements, including editor syntax highlighting and checkbox styles.

---

## Keybindings

### Navigation

| Key | Action |
|-----|--------|
| `q` | (Normal Mode) Exit editor and return to file list |
| `Enter` | Open selected file / execute action |

### Search

| Key | Action |
|-----|--------|
| `/` | Open fuzzy search (filename + content) |

### Editing (Vim-style)

| Key | Action |
|-----|--------|
| `i` | Insert mode (cursor stays) |
| `a` | Insert mode (cursor moves right) |
| `Esc` | Normal mode |
| `h` / `j` / `k` / `l` | Move cursor |
| `u` | Undo |
| `gt` / `gT` | Next / previous tab |
| `gs` | Toggle split view |
| `q` | Back to file list (auto-saves) |

### System

| Key | Action |
|-----|--------|
| `Ctrl+e` | Export current file to PDF |
| `Ctrl+p` | Command palette |
| `F11` | Toggle Zen mode |

### List Focus

| Key | Action |
|-----|--------|
| `j` / `k` | Move selection |
| `Enter` | Open note |
| `n` | Create new note |
| `r` | Rename file |
| `d` / `Delete` | Delete file |
| `c` | Edit config |
| `q` | Quit |

---

## License

This project is licensed under the **GPL v3**. See the [LICENSE](LICENSE) file for the full text.
