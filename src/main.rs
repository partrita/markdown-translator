mod cli;
mod translator;

use std::path::{Path, PathBuf};
use std::process;

use anyhow::Result;
use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};

use cli::{Cli, Commands, TranslateArgs};
use translator::{MarkdownTranslator, TranslationResult};

const BANNER: &str = r#"
╔═══════════════════════════════════════╗
║        Markdown Translator            ║
║     Powered by Google Gemini AI       ║
╚═══════════════════════════════════════╝
"#;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Translate(args)) => {
            if let Err(e) = handle_translate(args).await {
                eprintln!("{}", format!("\n❌ Error: {:#}", e).red());
                let err_str = format!("{:#}", e);
                if err_str.contains("API_KEY_INVALID") || err_str.contains("API key not valid") {
                    println!("{}", "Please check your Google Gemini API key".yellow());
                    println!("{}", "Get your API key from: https://aistudio.google.com/app/apikey".blue());
                } else if err_str.contains("429") || err_str.contains("RESOURCE_EXHAUSTED") || err_str.contains("ResourceExhausted") {
                    println!("{}", "💡 Tip: Gemini API Rate limit exceeded. Try adding `--delay 3000` or reducing `--chunk-size 4000`".yellow());
                }
                process::exit(1);
            }
        }
        Some(Commands::Languages) => {
            handle_languages();
        }
        Some(Commands::Setup) => {
            handle_setup();
        }
        None => {
            println!("{}", BANNER.cyan());
            let _ = Cli::parse_from(["md-translate", "--help"]);
        }
    }
}

async fn handle_translate(args: TranslateArgs) -> Result<()> {
    println!("{}", BANNER.cyan());

    // 1. Resolve API key
    let api_key = args
        .key
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .filter(|k| !k.trim().is_empty() && k != "your-google-gemini-api-key-here");

    let api_key = match api_key {
        Some(key) => key,
        None => {
            eprintln!("{}", "❌ Error: Google Gemini API key is required.".red());
            println!("{}", "Set GEMINI_API_KEY in your .env file, environment variable, or use --key option".yellow());
            println!("{}", "Get your API key from: https://aistudio.google.com/app/apikey".blue());
            process::exit(1);
        }
    };

    // 2. Resolve Gemini model
    let model_name = args
        .model
        .or_else(|| std::env::var("GEMINI_MODEL").ok())
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "gemini-3.6-flash".to_string());

    let chunk_size = args
        .chunk_size
        .or_else(|| std::env::var("TRANSLATION_CHUNK_SIZE").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(MarkdownTranslator::DEFAULT_CHUNK_SIZE);

    let delay_ms = args
        .delay
        .or_else(|| std::env::var("TRANSLATION_DELAY_MS").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(MarkdownTranslator::DEFAULT_DELAY_MS);

    let max_retries = args
        .retries
        .or_else(|| std::env::var("TRANSLATION_MAX_RETRIES").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(MarkdownTranslator::DEFAULT_MAX_RETRIES);

    let translator = MarkdownTranslator::new(api_key, model_name, chunk_size, delay_ms, max_retries);

    let input_pattern = args.input.trim();
    let is_wildcard = input_pattern.contains('*') || input_pattern.contains('?');

    // Check glob matches
    let matched_paths = match glob::glob(input_pattern) {
        Ok(paths) => {
            let mut list = Vec::new();
            for entry in paths.flatten() {
                let s = entry.to_string_lossy();
                if !s.contains("node_modules") && !s.contains(".git") && !s.contains("target") {
                    list.push(entry);
                }
            }
            list
        }
        Err(_) => Vec::new(),
    };

    let is_batch = is_wildcard || matched_paths.len() > 1;

    if is_batch {
        // Batch Translation Mode
        let output_dir_str = match &args.output_dir {
            Some(d) => d,
            None => {
                eprintln!("{}", "❌ Error: --output-dir is required for batch translation".red());
                println!("{}", "Use -d or --output-dir to specify the target directory".yellow());
                process::exit(1);
            }
        };

        let output_dir = Path::new(output_dir_str);

        println!("{}", "📋 Batch Translation Details:".blue());
        println!("{}", format!("   Pattern:   {}", input_pattern).bright_black());
        println!("{}", format!("   Output:    {}", output_dir.display()).bright_black());
        println!("{}", format!("   Language:  {}", args.language).bright_black());
        println!("{}", format!("   Structure: {}", if args.flat { "Flat" } else { "Preserved" }).bright_black());
        println!();

        let valid_files: Vec<PathBuf> = matched_paths
            .into_iter()
            .filter(|p| {
                if let Some(ext) = p.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    ext == "md" || ext == "markdown" || ext == "mdx" || ext == "qmd"
                } else {
                    false
                }
            })
            .collect();

        if valid_files.is_empty() {
            eprintln!("{}", format!("❌ Error: No markdown or quarto (.qmd) files found matching pattern: {}", input_pattern).red());
            process::exit(1);
        }

        println!("{}", format!("Found {} file(s) to translate\n", valid_files.len()).green());

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));

        let total_files = valid_files.len();
        let mut results: Vec<TranslationResult> = Vec::new();

        for (idx, file_path) in valid_files.iter().enumerate() {
            let file_num = idx + 1;
            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();

            let target_suffix = args.suffix.clone().unwrap_or_else(|| {
                args.language.to_lowercase().replace(' ', "_")
            });

            // Calculate output path
            let output_path = if args.flat {
                let file_stem = file_path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = file_path.extension().unwrap_or_default().to_string_lossy();
                let new_file_name = if !target_suffix.is_empty() {
                    format!("{}_{}.{}", file_stem, target_suffix, ext)
                } else {
                    format!("{}.{}", file_stem, ext)
                };
                output_dir.join(new_file_name)
            } else {
                let rel_path = file_path.strip_prefix(".").unwrap_or(file_path);
                let parent = rel_path.parent().unwrap_or_else(|| Path::new(""));
                let file_stem = file_path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = file_path.extension().unwrap_or_default().to_string_lossy();
                let new_file_name = if !target_suffix.is_empty() {
                    format!("{}_{}.{}", file_stem, target_suffix, ext)
                } else {
                    format!("{}.{}", file_stem, ext)
                };
                output_dir.join(parent).join(new_file_name)
            };

            pb.set_message(format!("[{}/{}] {} - translating...", file_num, total_files, file_name));

            let pb_clone = pb.clone();
            let file_name_clone = file_name.to_string();
            let progress_cb = move |chunk: usize, total_chunks: usize| {
                pb_clone.set_message(format!(
                    "[{}/{}] {} - chunk {}/{}",
                    file_num, total_files, file_name_clone, chunk, total_chunks
                ));
            };

            match translator.translate_file(file_path, &output_path, &args.language, progress_cb).await {
                Ok(res) => {
                    results.push(res);
                }
                Err(err) => {
                    eprintln!("{}", format!("❌ Failed to translate {}: {:#}", file_path.display(), err).red());
                    results.push(TranslationResult {
                        input_path: file_path.clone(),
                        output_path,
                        target_language: args.language.clone(),
                        original_length: 0,
                        translated_length: 0,
                        error: Some(format!("{:#}", err)),
                    });
                }
            }
        }

        let successful = results.iter().filter(|r| r.error.is_none()).count();
        let failed = results.len() - successful;

        if failed == 0 {
            pb.finish_with_message("✅ All translations completed successfully!".green().to_string());
        } else {
            pb.finish_with_message(format!("⚠️  Translation completed with {} failures", failed).yellow().to_string());
        }

        println!("{}", "\n📊 Batch Translation Summary:".blue());
        println!("{}", format!("   Files processed:  {}", results.len()).bright_black());
        println!("{}", format!("   ✅ Successful:    {}", successful).green());
        if failed > 0 {
            println!("{}", format!("   ❌ Failed:        {}", failed).red());
        }
        println!("{}", format!("   📁 Output dir:    {}", output_dir.display()).bright_black());

    } else {
        // Single File Translation Mode
        let input_path = Path::new(input_pattern);

        if !input_path.exists() {
            eprintln!("{}", format!("❌ Error: Input file not found: {}", input_path.display()).red());
            process::exit(1);
        }

        let ext = input_path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if ext != "md" && ext != "markdown" && ext != "mdx" && ext != "qmd" {
            eprintln!("{}", "❌ Error: Input file must be a markdown or Quarto file (.md, .markdown, .mdx, or .qmd)".red());
            process::exit(1);
        }

        let target_suffix = args.suffix.clone().unwrap_or_else(|| {
            args.language.to_lowercase().replace(' ', "_")
        });

        let output_path: PathBuf = if let Some(out) = &args.output {
            PathBuf::from(out)
        } else if let Some(out_dir) = &args.output_dir {
            let file_stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
            let new_name = if !target_suffix.is_empty() {
                format!("{}_{}.{}", file_stem, target_suffix, ext)
            } else {
                format!("{}.{}", file_stem, ext)
            };
            Path::new(out_dir).join(new_name)
        } else {
            let parent = input_path.parent().unwrap_or_else(|| Path::new(""));
            let file_stem = input_path.file_stem().unwrap_or_default().to_string_lossy();
            let new_name = format!("{}_{}.{}", file_stem, target_suffix, ext);
            parent.join(new_name)
        };

        if output_path.exists() {
            println!("{}", format!("⚠️  Warning: Output file already exists: {}", output_path.display()).yellow());
        }

        println!("{}", "📋 Translation Details:".blue());
        println!("{}", format!("   Input:    {}", input_path.display()).bright_black());
        println!("{}", format!("   Output:   {}", output_path.display()).bright_black());
        println!("{}", format!("   Language: {}", args.language).bright_black());
        println!();

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb.set_message("Initializing translation...");

        let pb_clone = pb.clone();
        let progress_cb = move |chunk: usize, total: usize| {
            pb_clone.set_message(format!("Translating chunk {}/{}...", chunk, total));
        };

        let result = translator
            .translate_file(input_path, &output_path, &args.language, progress_cb)
            .await?;

        pb.finish_with_message("✅ Translation completed successfully!".green().to_string());

        println!("{}", "\n📊 Summary:".blue());
        println!("{}", format!("   Original length:   {} characters", result.original_length).bright_black());
        println!("{}", format!("   Translated length: {} characters", result.translated_length).bright_black());
        println!("{}", format!("   Language:          {}", result.target_language).bright_black());
        println!("{}", format!("   Output file:       {}", result.output_path.display()).bright_black());
    }

    Ok(())
}

fn handle_languages() {
    println!("{}", BANNER.cyan());
    println!("{}", "🌍 Supported Languages:\n".blue());

    let languages = MarkdownTranslator::get_supported_languages();
    let columns = 3;
    let rows = (languages.len() + columns - 1) / columns;

    for i in 0..rows {
        let mut row_str = String::new();
        for j in 0..columns {
            let index = i + j * rows;
            if index < languages.len() {
                let formatted = format!("{:>2}. {:<15}", index + 1, languages[index]);
                row_str.push_str(&formatted);
            }
        }
        println!("{}", row_str);
    }

    println!("{}", "\n💡 Tip: You can also use any other language name that Gemini supports".yellow());
}

fn handle_setup() {
    println!("{}", BANNER.cyan());
    println!("{}", "🔧 Setup Guide:\n".blue());
    println!("{}", "1. Get your Google Gemini API key:".yellow());
    println!("{}", "   Visit: https://aistudio.google.com/app/apikey".bright_black());
    println!();
    println!("{}", "2. Configure .env file (Recommended):".yellow());
    println!("{}", "   Edit .env file in project root:".bright_black());
    println!("{}", "     GEMINI_API_KEY=your-api-key-here".white());
    println!("{}", "     GEMINI_MODEL=gemini-3.5-flash-lite".white());
    println!();
    println!("{}", "3. Or set environment variables:".yellow());
    println!("{}", "   Option A - Environment variable:".bright_black());
    println!("{}", "     export GEMINI_API_KEY=\"your-api-key-here\"".white());
    println!("{}", "     export GEMINI_MODEL=\"gemini-3.5-flash-lite\"".white());
    println!();
    println!("{}", "   Option B - Command line arguments:".bright_black());
    println!("{}", "     cargo run -- translate -i file.md -l Spanish --key your-api-key --model gemini-3.5-flash-lite".white());
    println!();
    println!("{}", "4. Start translating:".yellow());
    println!("{}", "     cargo run -- translate -i README.md -l Korean".white());
    println!("{}", "     ./target/release/md-translate translate -i README.md -l Korean".white());
    println!();
    println!("{}", "📚 For more help: cargo run -- --help".blue());
}
