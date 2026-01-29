# Sweeper

A fast, safe disk cleanup utility for developers built with Rust, GPUI, and adabraka-ui.

## Features

- **Smart Detection**: Automatically finds dev artifacts (node_modules, target/, build/), caches, and temporary files
- **Confidence Scores**: Each item shows a safety rating (70-98%) based on category and age
- **Stale Detection**: Projects not modified in 90+ days are highlighted for safe cleanup
- **Safe Cleanup**: Preview everything before deletion - moves to trash by default for easy undo
- **Cross-Platform**: Native support for macOS, Linux, and Windows
- **Beautiful UI**: Modern dark theme with intuitive navigation using adabraka-ui components
- **Fast Scanning**: Parallel scanning with rayon for rapid discovery
- **Memory Efficient**: Streaming results instead of loading everything into memory

## What Sweeper Finds

| Category | Examples |
|----------|----------|
| **Dev Artifacts** | `node_modules/`, `target/` (Rust), `.build/` (Swift), `Pods/`, `DerivedData/` |
| **Package Caches** | npm, yarn, pnpm, cargo, gradle, maven, pip, gem, composer |
| **IDE Caches** | VS Code, Cursor, Windsurf, Zed, JetBrains |
| **Browser Caches** | Chrome, Safari, Firefox, Edge, Brave |
| **System Caches** | Library/Caches, .cache, temp files, logs |

## Installation

### Build from Source

Requires [Rust 1.70+](https://rustup.rs/)

```bash
# Clone the repository
git clone https://github.com/augani/sweeper-rust.git
cd sweeper-rust

# Build release version
cargo build --release

# Run
./target/release/sweeper
```

## Usage

Launch the application and click **Rescan** to scan your system for cleanup candidates.

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `R` | Rescan |
| `A` | Select All |
| `D` | Delete Selected |
| `Esc` | Cancel/Close Dialog |
| `Q` | Quit |

## Safety

Sweeper is designed with safety in mind:

- **Preview First**: All items shown before deletion with size and category
- **No System Files**: Never touches OS-critical directories
- **Confidence Scores**: Each item shows 70-98% safety rating
- **Trash by Default**: Moves to system trash instead of permanent deletion

## Tech Stack

- **Rust** - Memory-safe systems programming
- **GPUI** - GPU-accelerated UI framework from Zed
- **adabraka-ui** - 80+ polished UI components
- **rayon** - Data parallelism for fast scanning
- **walkdir** - Efficient directory traversal
- **trash** - Cross-platform trash support

## License

MIT License - see [LICENSE](LICENSE) for details.
