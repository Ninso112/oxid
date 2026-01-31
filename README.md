# Oxid

[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-orange.svg)](https://www.rust-lang.org)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

**Oxid** is a keyboard-driven terminal user interface (TUI) for editing and managing Markdown notes. Built in Rust with [Ratatui](https://ratatui.rs), it offers a fast, distraction-free workflow with Vim-style editing, configurable keybindings, and productivity features such as daily notes and a global task board.

---

## Features

### Configurable Keybindings

Every action can be rebound. In `config.toml` you define keybindings as strings (e.g. `"ctrl-s"`, `"alt-d"`, `"enter"`) and map them to actions. The status bar and command palette show the keys currently in use, so you can adapt Oxid to your muscle memory without touching the code.

### Productivity Tools

- **Daily Notes (Journaling)** — Open or create today’s note in one keystroke. Notes are stored in a configurable folder (e.g. `journal`) with filenames like `YYYY-MM-DD.md`.
- **Global Task Board** — View all unchecked tasks (`- [ ]`) across your workspace in one list. Jump to the file and line of any task with Enter.

### Core Features

- **Syntax highlighting** — Markdown and code blocks in the editor; headers, lists, and checkboxes are styled.
- **Search with highlighting** — Fuzzy search over filenames and content; matches are highlighted as you type.
- **File tree** — Side panel with full directory navigation, folders-first sorting, and **folder creation** (e.g. new directory in the current path).
- **Mouse support** — Click to focus and select in the file list and UI where applicable.

### Additional Capabilities

- **Command palette** — Quick access to rename, delete, insert date, toggle zen/split, Git push, and PDF export.
- **Wiki-links** — `[[Page]]`-style links; follow with Enter or a dedicated key; backlinks panel when enabled.
- **Tag Explorer** — Browse `#tags` and filter files by tag.
- **Tabs and split view** — Multiple files open; side-by-side split with configurable focus key.
- **Zen mode** — Hide file tree and preview for full-screen editing.
- **Auto-save** — Optional save after a configurable idle interval with a status indicator.
- **PDF export** — Export the current file to PDF via Pandoc (optional dependency).
- **Theming** — Colors and styles via `theme.toml` (XDG config directory).

---

## Installation

**Prerequisites:** Rust toolchain (e.g. 1.70+). Optional: [Pandoc](https://pandoc.org) for PDF export.

```bash
git clone https://github.com/Ninso112/oxid.git
cd oxid
cargo run --release
```

The release binary is produced at `target/release/oxid`. To install it system-wide:

```bash
cargo install --path .
```

### Arch Linux (AUR)

Oxid is available in the [Arch User Repository](https://aur.archlinux.org/packages/oxid-git) as `oxid-git`:

```bash
yay -S oxid-git
# or
paru -S oxid-git
```

---

## Configuration

Configuration lives under the XDG base directory. On first run, Oxid creates `~/.config/oxid/` and writes default `config.toml` and `theme.toml` if missing. **Every visual and behavioral aspect** can be tuned in `config.toml` (plus `theme.toml` for full color control).

### config.toml — The Holy Grail of Customization

You can override only what you need; defaults apply for the rest. All colors accept **hex** (`#RRGGBB`, `#RGB`) or **named** colors (e.g. `white`, `dark_gray`).

#### Example: Catppuccin Mocha–style config

```toml
notes_directory = "~/Documents/Notes"
daily_notes_folder = "journal"

[theme]
# Main editor and status bar (overrides theme.toml for these)
background = "#1e1e2e"
foreground = "#cdd6f4"
cursor = "#f5e0dc"
selection = "#a6e3a1"
statusbar_bg = "#313244"
statusbar_fg = "#cdd6f4"
border_color = "#89b4fa"

[editor]
typewriter_mode = false
enable_spellcheck = false
spellcheck_languages = ["en"]
show_backlinks = true
syntax_highlighting = true
auto_save = true
auto_save_interval = 30
line_numbers = true
rel_line_numbers = false
tab_width = 4
mouse_support = true

[ui]
border_style = "rounded"
icons = true
show_hidden = false

[keys]
quit = "q"
zen_mode = "f11"
search = "/"
command_palette = "ctrl-p"
daily_note = "alt-d"
task_board = "alt-t"
escape = "esc"
enter = "enter"
backspace = "backspace"
move_up = "k"
move_down = "j"
move_left = "h"
move_up_alt = "up"
move_down_alt = "down"
move_left_alt = "left"
delete = "delete"
list_create_note = "n"
list_create_dir = "shift-n"
list_tag_explorer = "shift-t"
list_rename = "r"
list_edit_config = "c"
list_delete = "d"
list_parent = "backspace"
list_parent_alt = "left"
editor_back = "q"
editor_pdf = "ctrl-e"
editor_backlinks = "ctrl-b"
editor_wiki_link = "ctrl-]"
editor_insert = "i"
editor_append = "a"
editor_split_focus = "tab"
```

#### Variable reference

| Section | Variable | Type | Description |
|--------|----------|------|-------------|
| **Root** | `notes_directory` | string | Path to your notes (supports `~`). |
| | `daily_notes_folder` | string | Folder for daily notes, relative to `notes_directory` (e.g. `journal`). |
| **[theme]** | `background` | color | Main editor background. |
| | `foreground` | color | Main text color. |
| | `cursor` | color | Cursor color. |
| | `selection` | color | Selected text / list highlight. |
| | `statusbar_bg` | color | Status bar (footer) background. |
| | `statusbar_fg` | color | Status bar text color. |
| | `border_color` | color | Window borders. |
| **[editor]** | `line_numbers` | bool | Show line numbers in gutter. |
| | `rel_line_numbers` | bool | Relative (hybrid) line numbers when line numbers are on. |
| | `tab_width` | integer | Tab width in spaces (1–16). |
| | `mouse_support` | bool | Enable mouse in editor. |
| | *(others)* | | `typewriter_mode`, `enable_spellcheck`, `spellcheck_languages`, `show_backlinks`, `syntax_highlighting`, `auto_save`, `auto_save_interval` — see defaults in generated config. |
| **[ui]** | `border_style` | string | `"rounded"`, `"double"`, `"thick"`, or `"plain"`. |
| | `icons` | bool | Nerd Fonts icons in file tree (`.md`, `.rs`, folders, etc.). |
| | `show_hidden` | bool | Show dotfiles in file tree. |
| **[keys]** | *action* | string | Key string: `"key"` or `"modifier-key"` (e.g. `"ctrl-s"`, `"alt-d"`, `"enter"`, `"f11"`). |

#### Minimalist build

- **No borders:** `[ui]` → `border_style = "plain"`.
- **No line numbers:** `[editor]` → `line_numbers = false`.
- **No icons:** `[ui]` → `icons = false`.
- **Dark, low-noise:** use a dark `[theme]` with muted `border_color` and `statusbar_bg`.

#### Power-user build

- **Relative line numbers:** `[editor]` → `line_numbers = true`, `rel_line_numbers = true`.
- **Icons:** `[ui]` → `icons = true` (requires a [Nerd Font](https://www.nerdfonts.com/) in your terminal).
- **Visible borders:** `border_style = "rounded"` or `"double"`.
- **Bright theme:** e.g. Catppuccin/Dracula hex colors in `[theme]` (see example above).

### theme.toml

For **full** control over every UI color (lists, preview, search highlight, markdown syntax, etc.), edit `theme.toml` in `~/.config/oxid/`. Hex (`#RRGGBB`) and named colors are supported. Values in `config.toml` `[theme]` override the corresponding colors from `theme.toml` for editor, status bar, and borders.

---

## Keybindings Reference

Default keybindings. All of these can be overridden in `config.toml` under `[keys]`.

### General

| Key       | Action                |
|----------|------------------------|
| `q`      | Quit (saves and exits) |
| `F11`    | Toggle zen mode        |
| `Ctrl+P` | Command palette        |

### Productivity

| Key     | Action                          |
|--------|----------------------------------|
| `Alt+D`| Open or create today’s daily note |
| `Alt+T`| Open global task board          |

### Navigation & Search

| Key   | Action                    |
|-------|----------------------------|
| `/`   | Open fuzzy search          |
| `j`/`k` | Move selection (list/panels) |
| `Enter` | Open file / run action  |
| `Backspace` / `Left` | Go to parent (file tree) |
| `Esc` | Close panel / back         |

### File Tree

| Key        | Action              |
|------------|----------------------|
| `n`        | Create new note      |
| `Shift+N`  | Create new folder    |
| `Shift+T`  | Tag Explorer         |
| `r`        | Rename file/folder   |
| `c`        | Edit config file     |
| `d`/`Del`  | Delete file/folder   |

### Editor (Vim-style)

| Key      | Action                    |
|----------|----------------------------|
| `i` / `a`| Insert mode               |
| `Esc`    | Normal mode                |
| `h`/`j`/`k`/`l` | Move cursor         |
| `q`      | Back to file list (saves)  |
| `Ctrl+E` | Export to PDF              |
| `Ctrl+B` | Focus backlinks panel      |
| `Ctrl+]` | Follow wiki-link           |
| `Tab`    | Switch focus (split view)  |

---

## Usage

### Getting Started

1. Set `notes_directory` and optionally `daily_notes_folder` in `~/.config/oxid/config.toml`.
2. Run Oxid; the file tree shows your notes directory.
3. Start the day: press **`Alt+D`** to open or create today’s daily note.
4. Check tasks: press **`Alt+T`** to open the global task board, then **Enter** on a task to jump to it.
5. Search: press **`/`** to fuzzy-search filenames and content; **Enter** opens the selected match.
6. Use **`Ctrl+P`** for the command palette (rename, delete, insert date, zen/split, Git push, PDF export).
7. Rebind any key in `[keys]` to match your preferences; the UI shows the current bindings.

### Workflow Example

Start your day by pressing **`Alt+D`** to open your daily note, then **`Alt+T`** to see all unchecked tasks across your vault. Use **`/`** to jump to any note by name or content, and **`q`** from the editor to save and return to the file list. Quit the app with **`q`** from the list (after saving).

---

## License

This project is licensed under the **GPL v3**. See the [LICENSE](LICENSE) file for the full text.
