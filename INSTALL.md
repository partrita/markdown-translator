# Quick Installation & Getting Started

## 🚀 Quick Start

### 1. Build Project (Rust)
```bash
cargo build --release
```

Binary will be generated at `./target/release/md-translate`.

### 2. Configure `.env` File
Create or edit `.env` in the root directory:
```bash
cp env.example .env
```

Edit `.env`:
```env
GEMINI_API_KEY=your-google-gemini-api-key-here
GEMINI_MODEL=gemini-3.5-flash-lite
```

### 3. Test with Sample File
```bash
cargo run -- translate -i examples/sample.md -l Spanish

# Or directly using release binary:
./target/release/md-translate translate -i examples/sample.md -l Spanish
```


## 📚 Available Commands

```bash
# Get help
./target/release/md-translate --help

# List supported languages
./target/release/md-translate languages

# Show setup guide
./target/release/md-translate setup

# Translate a file
./target/release/md-translate translate -i input.md -l TargetLanguage -o output.md
```

## 🎯 Examples

### Basic Translation
```bash
./target/release/md-translate translate -i README.md -l French
```

### Custom Output File
```bash
./target/release/md-translate translate -i docs/guide.md -l German -o docs/guide_de.md
```

### Using API Key and Model Arguments
```bash
./target/release/md-translate translate -i file.md -l Japanese --key AIzaSyC... --model gemini-3.5-flash-lite
```

## 🛠️ Install to System (Optional)

To use `md-translate` from anywhere:

```bash
cargo install --path .
```

Then you can use:
```bash
md-translate translate -i file.md -l Spanish
md-translate languages
md-translate setup
```


## 🔍 Troubleshooting

- **API Key Error**: Make sure your Gemini API key is valid and set correctly
- **File Not Found**: Check that your input file path is correct
- **Network Issues**: Ensure you have a stable internet connection

For more detailed information, see the main [README.md](README.md) file. 
