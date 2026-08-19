use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "md-translate")]
#[command(about = "Translate markdown, MDX, and Quarto (QMD) files using Google Gemini AI", long_about = None)]
#[command(version = "1.0.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Translate markdown, MDX, and Quarto files to specified language
    Translate(TranslateArgs),
    /// List supported languages
    Languages,
    /// Setup guide for Google Gemini API key
    Setup,
}

#[derive(Args, Debug, Clone)]
pub struct TranslateArgs {
    /// Input file path or glob pattern (e.g., "*.md", "*.qmd", "docs/**/*.qmd")
    #[arg(short = 'i', long = "input")]
    pub input: String,

    /// Target language (e.g., Spanish, French, German, Korean)
    #[arg(short = 'l', long = "language")]
    pub language: String,

    /// Output file path (for single file translation)
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,

    /// Output directory (for batch translation or single file)
    #[arg(short = 'd', long = "output-dir")]
    pub output_dir: Option<String>,

    /// Google Gemini API key (or set GEMINI_API_KEY env var in .env)
    #[arg(short = 'k', long = "key")]
    pub key: Option<String>,

    /// Google Gemini model name (or set GEMINI_MODEL env var in .env, default: gemini-3.5-flash-lite)
    #[arg(short = 'm', long = "model")]
    pub model: Option<String>,

    /// Use flat structure in output directory (default: preserve structure)
    #[arg(long = "flat")]
    pub flat: bool,

    /// Custom suffix for output files (default: language name)
    #[arg(long = "suffix")]
    pub suffix: Option<String>,
}
