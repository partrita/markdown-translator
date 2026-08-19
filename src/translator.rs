use std::path::{Path, PathBuf};
use std::time::Duration;
use anyhow::{bail, Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<Content>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ApiError {
    message: String,
    code: Option<u32>,
    status: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TranslationResult {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub target_language: String,
    pub original_length: usize,
    pub translated_length: usize,
    pub error: Option<String>,
}

pub struct MarkdownTranslator {
    api_key: String,
    model_name: String,
    client: reqwest::Client,
}

impl MarkdownTranslator {
    pub const DEFAULT_CHUNK_SIZE: usize = 800_000;

    pub fn new(api_key: String, model_name: String) -> Self {
        let normalized_model = if !model_name.starts_with("gemini-") && !model_name.starts_with("models/") {
            format!("gemini-{}", model_name)
        } else {
            model_name
        };

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        println!("{}", format!("Using model: {}", normalized_model).bright_black());

        Self {
            api_key,
            model_name: normalized_model,
            client,
        }
    }

    pub fn split_into_chunks(&self, content: &str, max_chunk_size: usize) -> Vec<String> {
        let lines = content.split('\n');
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for line in lines {
            if !current_chunk.is_empty() && current_chunk.len() + line.len() + 1 > max_chunk_size {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = line.to_string();
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push('\n');
                }
                current_chunk.push_str(line);
            }
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    pub fn create_translation_prompt(&self, text: &str, target_language: &str) -> String {
        format!(
            "Translate the following Markdown/MDX/Quarto (.qmd) document content to {}.\n\n\
            IMPORTANT INSTRUCTIONS:\n\
            1. Preserve ALL Markdown, MDX, and Quarto syntax and formatting (headers, links, code blocks, tables, callout blocks `:::`, shortcodes `{{{{< ... >}}}}`, JSX tags, etc.)\n\
            2. For YAML frontmatter (between `---` at the beginning):\n\
               - Translate user-facing string values (e.g., `title:`, `subtitle:`, `description:`, `abstract:`)\n\
               - Do NOT translate YAML keys (e.g., `format:`, `author:`, `date:`, `execute:`, `knitr:`, `jupyter:`)\n\
               - Do NOT translate boolean, numeric, or configuration values (e.g., `toc: true`, `echo: false`, `html`)\n\
            3. For Quarto/Markdown code blocks (```{{...}}```):\n\
               - Do NOT translate executable code itself, URLs, or file paths\n\
               - Do NOT translate cell execution option keys (e.g., `#| echo:`, `#| label:`, `#| warning:`)\n\
               - DO translate human-readable captions/titles in cell options (e.g., `#| fig-cap: \"...\"`, `#| tbl-cap: \"...\"`)\n\
               - DO translate human-readable code comments (e.g., `# ...`, `// ...`, `/* ... */`)\n\
            4. Do NOT translate inline code expressions (e.g., `r ...`, `python ...`) or Quarto shortcodes\n\
            5. Maintain the exact indentation, structure, and line breaks\n\
            6. If there are technical terms or proper nouns that should remain in English/original, keep them\n\
            7. Return ONLY the translated document content without wrapping the whole output in additional explanation or outer backticks\n\n\
            Document content to translate:\n\n\
            {}",
            target_language, text
        )
    }

    pub async fn translate_chunk(&self, chunk: &str, target_language: &str) -> Result<String> {
        let prompt = self.create_translation_prompt(chunk, target_language);
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_name, self.api_key
        );

        let request_body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read response body from Gemini API")?;

        if !status.is_success() {
            if let Ok(error_response) = serde_json::from_str::<GenerateContentResponse>(&body_text) {
                if let Some(err) = error_response.error {
                    bail!("Gemini API error ({}): {}", err.status.unwrap_or_default(), err.message);
                }
            }
            bail!("Gemini API returned status {}: {}", status, body_text);
        }

        let parsed_response: GenerateContentResponse = serde_json::from_str(&body_text)
            .with_context(|| format!("Failed to parse Gemini API response: {}", body_text))?;

        if let Some(candidates) = parsed_response.candidates {
            if let Some(first_candidate) = candidates.first() {
                if let Some(content) = &first_candidate.content {
                    if let Some(first_part) = content.parts.first() {
                        return Ok(first_part.text.trim().to_string());
                    }
                }
            }
        }

        if let Some(err) = parsed_response.error {
            bail!("Gemini API error: {}", err.message);
        }

        bail!("No text content found in Gemini response");
    }

    pub async fn translate_markdown<F>(
        &self,
        content: &str,
        target_language: &str,
        mut progress_callback: F,
    ) -> Result<String>
    where
        F: FnMut(usize, usize),
    {
        let chunks = self.split_into_chunks(content, Self::DEFAULT_CHUNK_SIZE);
        let mut translated_chunks = Vec::with_capacity(chunks.len());

        println!(
            "{}",
            format!(
                "Translating {} chunk(s) to {}...",
                chunks.len(),
                target_language
            )
            .blue()
        );

        for (i, chunk) in chunks.iter().enumerate() {
            progress_callback(i + 1, chunks.len());

            let translated_chunk = self.translate_chunk(chunk, target_language).await?;
            translated_chunks.push(translated_chunk);

            if i < chunks.len() - 1 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        let mut result = translated_chunks.join("\n\n");
        if !result.ends_with('\n') {
            result.push('\n');
        }

        Ok(result)
    }

    pub async fn translate_file<F>(
        &self,
        input_path: &Path,
        output_path: &Path,
        target_language: &str,
        progress_callback: F,
    ) -> Result<TranslationResult>
    where
        F: FnMut(usize, usize),
    {
        if !input_path.exists() {
            bail!("Input file does not exist: {}", input_path.display());
        }

        println!("{}", format!("Reading file: {}", input_path.display()).blue());
        let content = tokio::fs::read_to_string(input_path)
            .await
            .with_context(|| format!("Failed to read file: {}", input_path.display()))?;

        if content.trim().is_empty() {
            bail!("Input file is empty: {}", input_path.display());
        }

        let translated_content = self
            .translate_markdown(&content, target_language, progress_callback)
            .await?;

        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("Failed to create output directory: {}", parent.display())
            })?;
        }

        tokio::fs::write(output_path, &translated_content)
            .await
            .with_context(|| format!("Failed to write translated file: {}", output_path.display()))?;

        println!(
            "{}",
            format!("Translation completed: {}", output_path.display()).green()
        );

        Ok(TranslationResult {
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            target_language: target_language.to_string(),
            original_length: content.len(),
            translated_length: translated_content.len(),
            error: None,
        })
    }

    pub fn get_supported_languages() -> Vec<&'static str> {
        vec![
            "Spanish", "French", "German", "Italian", "Portuguese", "Dutch",
            "Russian", "Chinese", "Japanese", "Korean", "Arabic", "Hindi",
            "Turkish", "Polish", "Swedish", "Norwegian", "Danish", "Finnish",
            "Greek", "Hebrew", "Thai", "Vietnamese", "Indonesian", "Malay",
            "Ukrainian", "Czech", "Hungarian", "Romanian", "Bulgarian",
            "Croatian", "Serbian", "Slovak", "Slovenian", "Estonian",
            "Latvian", "Lithuanian", "Catalan", "Basque", "Welsh", "Irish",
        ]
    }
}
