# Markdown & Quarto Translator
 
[🇰🇷 한국어 버전 (Korean Version)](README.md)

A fast, high-performance command-line tool built with **Rust** that uses Google Gemini AI to translate Markdown, MDX, and Quarto (`.qmd`) documents to any specified language while preserving formatting and structure.

## Features

- 🌍 **Multi-language support** - Translate to 40+ languages
- 📝 **Markdown, MDX & Quarto (.qmd) aware** - Preserves markdown, MDX, and Quarto specific formatting (YAML frontmatter, cell options `#|`, callouts `:::`, shortcodes, headers, links, code blocks, tables, etc.)
- 🔄 **Smart chunking** - Handles large files by splitting content into manageable chunks (default: 6,000 characters)
- 🛡️ **Exponential backoff retry** - Automatically retries on network drops, timeouts, 429 rate limits, and transient server errors up to 3 times
- ⏱️ **Rate limit delay control** - Configurable delays between requests to stay within API quotas
- 🎯 **Selective translation** - Translates text and comments/captions while keeping executable code, options, and URLs intact
- 📂 **Batch processing** - Translate multiple files using glob patterns (e.g., `docs/**/*.qmd`, `docs/**/*.md`)
- 🏗️ **Structure preservation** - Maintain directory structure or flatten output as needed
- 📊 **Progress tracking** - Real-time progress indication with spinners for single files and batches
- 🎨 **Beautiful CLI** - Colorful, user-friendly command-line interface
- ⚡ **Fast & Efficient** - Asynchronous Rust implementation powered by `tokio`, `reqwest`, and Google Gemini AI

## Installation

### Prerequisites

- Rust (1.70+ / `cargo`)
- Google Gemini API key ([Get one here](https://aistudio.google.com/app/apikey))

### Build & Install

```bash
# Build release binary
cargo build --release

# (Optional) Install globally to PATH
cargo install --path .
```

The compiled binary will be located at `./target/release/md-translate`.

Or run directly with `cargo run`:

```bash
cargo run -- translate -i examples/sample.md -l Spanish
```

## Setup

### 1. Configure `.env` File (Recommended)

Copy the example environment file and configure your API key and model:

```bash
cp env.example .env
```

Edit `.env`:
```env
# Google Gemini API Configuration
GEMINI_API_KEY=your-google-gemini-api-key-here

# Optional: Gemini model name (default: gemini-3.5-flash-lite)
GEMINI_MODEL=gemini-3.5-flash-lite
```

### 2. Alternative: Environment Variables or CLI Arguments

**Option A: Environment Variables**
```bash
export GEMINI_API_KEY="your-api-key-here"
export GEMINI_MODEL="gemini-3.5-flash-lite"
```

**Option B: Command Line Arguments**
```bash
md-translate translate -i file.md -l Spanish --key your-api-key-here --model gemini-3.5-flash-lite
```

## Usage

### Basic Translation

```bash
# Translate README.md to Spanish
md-translate translate -i README.md -l Spanish

# Translate with custom output file
md-translate translate -i docs/guide.md -l French -o docs/guide_fr.md

# Translate using API key and model arguments
md-translate translate -i file.md -l German --key your-api-key --model gemini-3.5-flash-lite
```

### Batch Processing

The tool supports batch processing of multiple markdown and Quarto files using glob patterns:

```bash
# Translate all .qmd files in current directory
md-translate translate -i "*.qmd" -l Spanish -d ./spanish/

# Translate all files in docs folder and subfolders (preserves directory structure)
md-translate translate -i "docs/**/*.qmd" -l French -d ./translations/

# Batch translate with flat structure (no subdirectories)
md-translate translate -i "content/**/*.{md,qmd}" -l German -d ./output/ --flat

# Batch translate with custom suffix
md-translate translate -i "*.qmd" -l Japanese -d ./translated/ --suffix "ja"
```

### Available Commands

#### `translate` - Translate a Markdown, MDX, or Quarto file

```bash
md-translate translate [options]

Options:
  -i, --input <pattern>    Input file path or glob pattern (required)
                          Examples: "file.md", "document.qmd", "*.qmd", "docs/**/*.md"
  -l, --language <lang>    Target language (required)
  -o, --output <file>      Output file path (for single file translation)
  -d, --output-dir <dir>   Output directory (for batch translation or single file)
  -k, --key <apikey>       Google Gemini API key (optional if set in .env)
  -m, --model <model>      Google Gemini model name (optional if set in .env)
      --flat               Use flat structure in output directory (default: preserve structure)
      --suffix <suffix>    Custom suffix for output files (default: language name)
      --chunk-size <size>  Chunk size in characters for splitting large documents (default: 6000)
      --delay <ms>         Delay between API requests in milliseconds (default: 1500)
      --retries <count>    Maximum retries on rate limit or network error (default: 3)
```

#### `languages` - List supported languages

```bash
md-translate languages
```

#### `setup` - Show setup guide

```bash
md-translate setup
```

#### `--help` - Show help

```bash
md-translate --help
```

## Supported Languages

The tool supports 40+ languages including:

- **European**: Spanish, French, German, Italian, Portuguese, Dutch, Russian, Polish, Swedish, Norwegian, Danish, Finnish, Greek, Ukrainian, Czech, Hungarian, Romanian, Bulgarian, Croatian, Serbian, Slovak, Slovenian, Estonian, Latvian, Lithuanian, Catalan, Basque, Welsh, Irish
- **Asian**: Chinese, Japanese, Korean, Hindi, Thai, Vietnamese, Indonesian, Malay
- **Middle Eastern**: Arabic, Hebrew, Turkish
- *Any other language supported by Google Gemini*

## Examples

### Single File Translation

#### Example 1: Basic Translation
```bash
md-translate translate -i README.md -l Spanish
```
**Output**: Creates `README_spanish.md` with Spanish translation.

#### Example 2: Custom Output Path
```bash
md-translate translate -i docs/api.md -l French -o docs/fr/api.md
```
**Output**: Creates `docs/fr/api.md` with French translation.

#### Example 3: Large File Translation
The tool automatically chunks large files:
```bash
md-translate translate -i large-document.md -l Japanese
```

### Batch Translation

#### Example 4: Translate All Markdown Files
```bash
md-translate translate -i "*.md" -l Spanish -d ./spanish/
```

#### Example 5: Recursive Translation with Structure Preservation
```bash
md-translate translate -i "docs/**/*.md" -l French -d ./translations/
```

```
docs/
├── guide.md
├── api/
│   └── reference.md
└── tutorials/
    └── getting-started.md

# Becomes:
translations/
├── guide_french.md
├── api/
│   └── reference_french.md
└── tutorials/
    └── getting-started_french.md
```

#### Example 6: Flat Structure Batch Translation
```bash
md-translate translate -i "content/**/*.md" -l German -d ./output/ --flat
```

## What Gets Translated

✅ **Translated**:
- Heading text
- Paragraph text
- List items
- Table content
- Link text
- Image alt text
- Quote text
- Comments inside code blocks

❌ **Preserved**:
- Code blocks and inline code
- URLs and file paths
- Markdown syntax characters
- HTML tags
- Mathematical expressions
- Technical terms and proper nouns

## Project Structure

```
markdown-translator/
├── Cargo.toml           # Rust package configuration & dependencies
├── src/
│   ├── main.rs          # CLI entry point and execution flow
│   ├── cli.rs           # Clap command line argument definitions
│   └── translator.rs    # Translation engine and Gemini API client
├── .env                 # Environment configuration (API Key & Model)
├── env.example          # Example environment configuration
├── examples/            # Sample markdown files
├── README.md            # Korean documentation
└── README.en.md         # English documentation
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
