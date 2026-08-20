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
    pub chunk_size: usize,
    pub delay_ms: u64,
    pub max_retries: u32,
}

impl MarkdownTranslator {
    pub const DEFAULT_CHUNK_SIZE: usize = 6_000;
    pub const DEFAULT_DELAY_MS: u64 = 1_500;
    pub const DEFAULT_MAX_RETRIES: u32 = 3;

    pub fn new(
        api_key: String,
        model_name: String,
        chunk_size: usize,
        delay_ms: u64,
        max_retries: u32,
    ) -> Self {
        let model_clean = model_name.strip_prefix("models/").unwrap_or(&model_name);
        let normalized_model = if !model_clean.starts_with("gemini-") {
            format!("gemini-{}", model_clean)
        } else {
            model_clean.to_string()
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
            chunk_size,
            delay_ms,
            max_retries,
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

    async fn translate_chunk_once(&self, chunk: &str, target_language: &str) -> Result<String> {
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
            .with_context(|| format!("Failed to send HTTP request to Gemini API (model: {})", self.model_name))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read response body from Gemini API")?;

        if !status.is_success() {
            if let Ok(error_response) = serde_json::from_str::<GenerateContentResponse>(&body_text) {
                if let Some(err) = error_response.error {
                    bail!(
                        "Gemini API error [{}] {}: {}",
                        status.as_u16(),
                        err.status.unwrap_or_default(),
                        err.message
                    );
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

    pub async fn translate_chunk(&self, chunk: &str, target_language: &str) -> Result<String> {
        let max_attempts = self.max_retries + 1;
        let mut last_err = None;

        for attempt in 1..=max_attempts {
            match self.translate_chunk_once(chunk, target_language).await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    let err_str = format!("{:#}", err);
                    let is_retryable = err_str.contains("429")
                        || err_str.contains("503")
                        || err_str.contains("500")
                        || err_str.contains("502")
                        || err_str.contains("504")
                        || err_str.contains("RESOURCE_EXHAUSTED")
                        || err_str.contains("ResourceExhausted")
                        || err_str.contains("timed out")
                        || err_str.contains("timeout")
                        || err_str.contains("connection closed")
                        || err_str.contains("Connection reset")
                        || err_str.contains("Failed to send HTTP request");

                    if attempt < max_attempts && is_retryable {
                        let backoff_secs = 2u64.pow(attempt - 1) * 2; // 2s, 4s, 8s...
                        eprintln!(
                            "{}",
                            format!(
                                "   ⚠️ Request failed (attempt {}/{}): {}. Retrying in {}s...",
                                attempt, max_attempts, err, backoff_secs
                            )
                            .yellow()
                        );
                        tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                        last_err = Some(err);
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded")))
    }

    #[allow(dead_code)]
    pub async fn translate_markdown<F>(
        &self,
        content: &str,
        target_language: &str,
        mut progress_callback: F,
    ) -> Result<String>
    where
        F: FnMut(usize, usize),
    {
        let chunks = self.split_into_chunks(content, self.chunk_size);
        let mut translated_chunks = Vec::with_capacity(chunks.len());

        println!(
            "{}",
            format!(
                "Translating {} chunk(s) (chunk size: ~{} chars) to {}...",
                chunks.len(),
                self.chunk_size,
                target_language
            )
            .blue()
        );

        for (i, chunk) in chunks.iter().enumerate() {
            progress_callback(i + 1, chunks.len());

            let translated_chunk = self.translate_chunk(chunk, target_language).await?;
            translated_chunks.push(translated_chunk);

            if i < chunks.len() - 1 {
                tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
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
        mut progress_callback: F,
    ) -> Result<TranslationResult>
    where
        F: FnMut(usize, usize),
    {
        use tokio::io::AsyncWriteExt;

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

        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.with_context(|| {
                    format!("Failed to create output directory: {}", parent.display())
                })?;
            }
        }

        let temp_path = PathBuf::from(format!("{}.tmp", output_path.display()));
        let mut temp_file = tokio::fs::File::create(&temp_path)
            .await
            .with_context(|| format!("Failed to create temporary file: {}", temp_path.display()))?;

        let chunks = self.split_into_chunks(&content, self.chunk_size);
        let total_chunks = chunks.len();

        println!(
            "{}",
            format!(
                "Translating {} chunk(s) (chunk size: ~{} chars) to {}...",
                total_chunks,
                self.chunk_size,
                target_language
            )
            .blue()
        );

        let mut translated_length = 0;

        for (i, chunk) in chunks.iter().enumerate() {
            progress_callback(i + 1, total_chunks);

            match self.translate_chunk(chunk, target_language).await {
                Ok(translated_chunk) => {
                    if i > 0 {
                        temp_file.write_all(b"\n\n").await.with_context(|| {
                            format!("Failed to write separator to temporary file: {}", temp_path.display())
                        })?;
                        translated_length += 2;
                    }
                    temp_file.write_all(translated_chunk.as_bytes()).await.with_context(|| {
                        format!("Failed to write chunk to temporary file: {}", temp_path.display())
                    })?;
                    temp_file.flush().await.with_context(|| {
                        format!("Failed to flush temporary file: {}", temp_path.display())
                    })?;
                    translated_length += translated_chunk.len();

                    if i < total_chunks - 1 {
                        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
                    }
                }
                Err(err) => {
                    let _ = temp_file.flush().await;
                    drop(temp_file);
                    eprintln!(
                        "{}",
                        format!(
                            "⚠️  Translation interrupted at chunk {}/{}. Partial translation saved to: {}",
                            i + 1,
                            total_chunks,
                            temp_path.display()
                        )
                        .yellow()
                    );
                    return Err(err.context(format!(
                        "Translation failed at chunk {}/{}. Partial translation preserved in: {}",
                        i + 1,
                        total_chunks,
                        temp_path.display()
                    )));
                }
            }
        }

        // Ensure newline at end of file
        temp_file.write_all(b"\n").await.ok();
        temp_file.flush().await.ok();
        drop(temp_file);
        translated_length += 1;

        // Atomically replace target file with temp file (or fallback to copy)
        if let Err(_) = tokio::fs::rename(&temp_path, output_path).await {
            tokio::fs::copy(&temp_path, output_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to copy temporary file {} to {}",
                        temp_path.display(),
                        output_path.display()
                    )
                })?;
            let _ = tokio::fs::remove_file(&temp_path).await;
        }

        println!(
            "{}",
            format!("Translation completed: {}", output_path.display()).green()
        );

        Ok(TranslationResult {
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            target_language: target_language.to_string(),
            original_length: content.len(),
            translated_length,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_chunks() {
        let translator = MarkdownTranslator::new(
            "dummy_key".to_string(),
            "gemini-3.6-flash".to_string(),
            50,
            0,
            0,
        );

        let markdown = "# Title\n\nThis is paragraph one.\n\nThis is paragraph two.\n\nThis is paragraph three.";
        let chunks = translator.split_into_chunks(markdown, 30);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= 50);
        }
    }

    #[test]
    fn test_model_normalization() {
        let translator = MarkdownTranslator::new(
            "dummy_key".to_string(),
            "models/gemini-pro".to_string(),
            6000,
            0,
            0,
        );
        assert_eq!(translator.model_name, "gemini-pro");

        let translator2 = MarkdownTranslator::new(
            "dummy_key".to_string(),
            "custom-model".to_string(),
            6000,
            0,
            0,
        );
        assert_eq!(translator2.model_name, "gemini-custom-model");
    }
}
