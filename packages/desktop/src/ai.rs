//! LLM assistance for the terminal: turn a request into a shell command, or
//! explain selected output. Requests go out from here so the API key never
//! reaches the webview and secrets are redacted in one place.

use std::time::Duration;

use serde_json::{json, Value};

use crate::app::AppHandle;
use crate::store::settings::{delete_setting, get_setting, set_setting};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_TOKENS: u32 = 1024;
const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com/v1";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const OPENCODE_GO_BASE_URL: &str = "https://opencode.ai/zen/go/v1";

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    Anthropic,
    /// Any OpenAI-compatible chat completions API: OpenAI, Ollama, LM Studio, …
    OpenaiCompatible,
    /// One key for many vendors' models, each served in its own endpoint format.
    OpencodeGo,
}

/// The wire format a model is reached through.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiApiFormat {
    /// `/messages`
    Anthropic,
    /// `/chat/completions`
    Openai,
    /// `/responses`, OpenAI's Responses API
    Responses,
    /// Google's `generateContent`
    Gemini,
}

impl AiApiFormat {
    /// OpenCode Go serves MiniMax, Qwen and Claude in Anthropic format, GPT, Grok and
    /// Muse through Responses, Gemini natively, and the rest as chat completions.
    // ponytail: name-prefix guess; the settings let users pick the format when it is wrong.
    fn for_opencode_model(model: &str) -> Self {
        let starts = |prefixes: &[&str]| prefixes.iter().any(|prefix| model.starts_with(prefix));
        if starts(&["minimax", "qwen", "claude"]) {
            Self::Anthropic
        } else if starts(&["gpt", "grok", "muse"]) {
            Self::Responses
        } else if starts(&["gemini"]) {
            Self::Gemini
        } else {
            Self::Openai
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiTask {
    /// `input` is what the user wants done; reply is one shell command.
    Command,
    /// `input` is selected terminal text; reply is a short explanation.
    Explain,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiAskInput {
    pub provider: AiProvider,
    pub model: String,
    pub base_url: Option<String>,
    /// Only OpenCode Go uses this; `None` picks the format from the model name.
    pub api_format: Option<AiApiFormat>,
    pub task: AiTask,
    pub input: String,
    /// Recent terminal output, so the model can see the shell, OS and cwd.
    pub context: Option<String>,
    /// Extra question about an explained selection.
    pub question: Option<String>,
    /// Reply language, e.g. "繁體中文" or "English".
    pub language: String,
}

pub async fn ai_set_api_key(
    app: AppHandle,
    provider: AiProvider,
    api_key: Option<String>,
) -> Result<(), String> {
    let key = secret_name(provider).to_string();
    match api_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(value) => set_setting(shared::Setting { key, value }, app.state())
            .await
            .map(|_| ()),
        None => delete_setting(key, app.state()).await,
    }
}

pub async fn ai_has_api_key(app: AppHandle, provider: AiProvider) -> Result<bool, String> {
    Ok(api_key(&app, provider).await?.is_some())
}

pub async fn ai_ask(app: AppHandle, input: AiAskInput) -> Result<String, String> {
    let key = api_key(&app, input.provider).await?;
    if input.provider != AiProvider::OpenaiCompatible && key.is_none() {
        return Err("Add an API key in Settings first".to_string());
    }
    let model = input.model.trim();
    if model.is_empty() {
        return Err("Choose a model in Settings first".to_string());
    }
    let (format, default_base) = match input.provider {
        AiProvider::Anthropic => (AiApiFormat::Anthropic, ANTHROPIC_BASE_URL),
        AiProvider::OpenaiCompatible => (AiApiFormat::Openai, OPENAI_BASE_URL),
        AiProvider::OpencodeGo => (
            input
                .api_format
                .unwrap_or_else(|| AiApiFormat::for_opencode_model(model)),
            OPENCODE_GO_BASE_URL,
        ),
    };
    let base = input
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .unwrap_or(default_base)
        .trim_end_matches('/')
        .to_string();
    let (system, user) = build_prompt(&input);

    let mut headers = reqwest::header::HeaderMap::new();
    if input.provider == AiProvider::OpencodeGo {
        // OpenCode Go routes and caches per session id.
        let session = uuid::Uuid::new_v4().to_string();
        headers.insert(
            "x-opencode-session",
            session
                .parse()
                .map_err(|_| "invalid session id".to_string())?,
        );
    }
    // OpenCode Go asks clients to name themselves rather than send a library default.
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("noverterm/", env!("CARGO_PKG_VERSION")))
        .default_headers(headers)
        .build()
        .map_err(|error| error.to_string())?;
    let key = key.unwrap_or_default();

    let reply = match format {
        AiApiFormat::Anthropic => {
            let body = read_json(
                client
                    .post(format!("{base}/messages"))
                    .header("x-api-key", &key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&json!({
                        "model": model,
                        "max_tokens": MAX_TOKENS,
                        "system": system,
                        "messages": [{ "role": "user", "content": user }],
                    }))
                    .send()
                    .await,
            )
            .await?;
            join_text(&body["content"], "text")
        }
        AiApiFormat::Openai => {
            let mut request = client
                .post(format!("{base}/chat/completions"))
                .json(&json!({
                    "model": model,
                    "max_tokens": MAX_TOKENS,
                    "messages": [
                        { "role": "system", "content": system },
                        { "role": "user", "content": user },
                    ],
                }));
            // Local servers such as Ollama need no key.
            if !key.is_empty() {
                request = request.bearer_auth(&key);
            }
            let body = read_json(request.send().await).await?;
            body["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        }
        AiApiFormat::Responses => {
            let body = read_json(
                client
                    .post(format!("{base}/responses"))
                    .bearer_auth(&key)
                    .json(&json!({
                        "model": model,
                        "max_output_tokens": MAX_TOKENS,
                        "instructions": system,
                        "input": user,
                    }))
                    .send()
                    .await,
            )
            .await?;
            body["output"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| join_text(&item["content"], "text"))
                .collect::<Vec<_>>()
                .join("")
        }
        AiApiFormat::Gemini => {
            let body = read_json(
                client
                    .post(format!(
                        "{}/v1beta/models/{model}:generateContent",
                        gemini_base_url(&base)
                    ))
                    .header("x-goog-api-key", &key)
                    .json(&json!({
                        "systemInstruction": { "parts": [{ "text": system }] },
                        "contents": [{ "role": "user", "parts": [{ "text": user }] }],
                        "generationConfig": { "maxOutputTokens": MAX_TOKENS },
                    }))
                    .send()
                    .await,
            )
            .await?;
            join_text(&body["candidates"][0]["content"]["parts"], "text")
        }
    };

    let reply = reply.trim();
    if reply.is_empty() {
        return Err("The model returned an empty reply".to_string());
    }
    Ok(match input.task {
        AiTask::Command => strip_code_fence(reply),
        AiTask::Explain => reply.to_string(),
    })
}

/// Concatenates `field` of every block in a JSON array of content blocks.
fn join_text(blocks: &Value, field: &str) -> String {
    blocks
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| block[field].as_str())
        .collect::<Vec<_>>()
        .join("")
}

/// OpenCode Go serves Gemini under `/inference/go/google` rather than next to
/// the other formats; any other base URL is used as is.
fn gemini_base_url(base_url: &str) -> String {
    match base_url.strip_suffix("/zen/go/v1") {
        Some(origin) => format!("{origin}/inference/go/google"),
        None => base_url.to_string(),
    }
}

/// The settings key holding a provider's API key.
fn secret_name(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::Anthropic => "noverterm-ai-key.anthropic",
        AiProvider::OpenaiCompatible => "noverterm-ai-key.openai-compatible",
        AiProvider::OpencodeGo => "noverterm-ai-key.opencode-go",
    }
}

async fn api_key(app: &AppHandle, provider: AiProvider) -> Result<Option<String>, String> {
    let setting = get_setting(secret_name(provider).to_string(), app.state()).await?;
    Ok(setting.map(|setting| setting.value))
}

async fn read_json(response: reqwest::Result<reqwest::Response>) -> Result<Value, String> {
    let response = response.map_err(|error| format!("Could not reach the AI provider: {error}"))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|error| format!("AI provider sent an unreadable reply: {error}"))?;
    if status.is_success() {
        return Ok(body);
    }
    let message = body["error"]["message"]
        .as_str()
        .or_else(|| body["error"].as_str())
        .unwrap_or("request failed");
    Err(format!("AI provider error ({status}): {message}"))
}

fn build_prompt(input: &AiAskInput) -> (String, String) {
    let context = input
        .context
        .as_deref()
        .map(redact)
        .filter(|context| !context.trim().is_empty())
        .map(|context| format!("Recent terminal output:\n<terminal>\n{context}\n</terminal>\n\n"))
        .unwrap_or_default();
    match input.task {
        AiTask::Command => (
            "You turn a request into one shell command for the terminal shown. \
             Infer the OS and shell from the terminal output when you can. \
             Reply with the command only: no explanation, no code fence. \
             Prefer a single line; chain steps with && when needed. \
             If the request is unsafe or impossible, reply with a shell comment \
             starting with # that says why."
                .to_string(),
            format!("{context}Request: {}", redact(&input.input)),
        ),
        AiTask::Explain => (
            format!(
                "You explain terminal output to a developer. Be concise: say what it \
                 means, the likely cause of any error, and the next command to try. \
                 Use plain text with short paragraphs or lists, no headings. \
                 Reply in {}.",
                input.language
            ),
            format!(
                "{context}Selected text:\n<selection>\n{}\n</selection>{}",
                redact(&input.input),
                input
                    .question
                    .as_deref()
                    .map(str::trim)
                    .filter(|question| !question.is_empty())
                    .map(|question| format!("\n\nQuestion: {}", redact(question)))
                    .unwrap_or_default()
            ),
        ),
    }
}

/// Masks secrets that commonly show up in terminal output before it leaves
/// the machine.
// ponytail: pattern-based; it catches common key formats, not every secret.
fn redact(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_private_key = false;
    for line in text.split_inclusive('\n') {
        if line.contains("-----BEGIN") && line.contains("PRIVATE KEY") {
            in_private_key = true;
            out.push_str("[redacted private key]\n");
        }
        if in_private_key {
            if line.contains("-----END") {
                in_private_key = false;
            }
            continue;
        }
        out.push_str(&redact_line(line));
    }
    out
}

fn redact_line(line: &str) -> String {
    const SECRET_NAMES: [&str; 7] = [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "authorization",
    ];
    const KEY_PREFIXES: [&str; 6] = ["sk-", "ghp_", "gho_", "github_pat_", "xox", "AKIA"];

    let mut words: Vec<String> = Vec::new();
    let mut mask_next = false;
    for word in line.split_inclusive(char::is_whitespace) {
        let trimmed = word.trim_end();
        let trailing = &word[trimmed.len()..];
        let lower = trimmed.to_ascii_lowercase();
        let masked = if mask_next && !trimmed.is_empty() {
            mask_next = false;
            Some("[redacted]".to_string())
        } else if let Some((name, _)) = trimmed.split_once(['=', ':']).filter(|(name, value)| {
            !value.is_empty()
                && SECRET_NAMES
                    .iter()
                    .any(|secret| name.to_ascii_lowercase().contains(secret))
        }) {
            // `API_KEY=abc` keeps its name and separator: `API_KEY=[redacted]`.
            Some(format!(
                "{name}{}[redacted]",
                &trimmed[name.len()..=name.len()]
            ))
        } else if KEY_PREFIXES
            .iter()
            .any(|prefix| trimmed.starts_with(prefix) && trimmed.len() >= 20)
        {
            Some("[redacted]".to_string())
        } else {
            // `Authorization: Bearer xyz`, `password: xyz`, `--password xyz`
            if lower == "bearer"
                || SECRET_NAMES
                    .iter()
                    .any(|secret| lower.trim_start_matches('-').trim_end_matches(':') == *secret)
            {
                mask_next = true;
            }
            None
        };
        words.push(match masked {
            Some(masked) => format!("{masked}{trailing}"),
            None => word.to_string(),
        });
    }
    words.concat()
}

fn strip_code_fence(reply: &str) -> String {
    let reply = reply.trim();
    let Some(inner) = reply.strip_prefix("```") else {
        return reply.to_string();
    };
    let inner = inner.split_once('\n').map_or("", |(_, rest)| rest);
    inner.trim_end().trim_end_matches("```").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secrets() {
        let text = "\
$ export API_KEY=abc123
password: hunter2
curl -H 'Authorization: Bearer eyJhbGciOi' https://x
mysql --password secretpw -u root
key sk-ant-REDACTMEREDACTMEREDACTME
-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAA
-----END OPENSSH PRIVATE KEY-----
ls -la
";
        let redacted = redact(text);
        for secret in [
            "abc123",
            "hunter2",
            "eyJhbGciOi",
            "secretpw",
            "REDACTME",
            "b3BlbnNzaC1rZXk",
        ] {
            assert!(!redacted.contains(secret), "{secret} leaked:\n{redacted}");
        }
        assert!(redacted.contains("API_KEY=[redacted]"));
        assert!(redacted.contains("[redacted private key]"));
        assert!(
            redacted.ends_with("ls -la\n"),
            "ordinary lines are kept:\n{redacted}"
        );
    }

    #[test]
    fn opencode_go_picks_the_format_by_model_and_maps_gemini() {
        assert_eq!(
            AiApiFormat::for_opencode_model("minimax-m3"),
            AiApiFormat::Anthropic
        );
        assert_eq!(
            AiApiFormat::for_opencode_model("gpt-6-luna"),
            AiApiFormat::Responses
        );
        assert_eq!(
            AiApiFormat::for_opencode_model("gemini-4-argon"),
            AiApiFormat::Gemini
        );
        assert_eq!(
            AiApiFormat::for_opencode_model("glm-5.3"),
            AiApiFormat::Openai
        );
        assert_eq!(
            gemini_base_url("https://opencode.ai/zen/go/v1"),
            "https://opencode.ai/inference/go/google"
        );
        assert_eq!(gemini_base_url("http://proxy.test"), "http://proxy.test");
    }

    #[test]
    fn strips_a_code_fence_from_a_command_reply() {
        assert_eq!(strip_code_fence("```bash\nls -la\n```"), "ls -la");
        assert_eq!(strip_code_fence("df -h"), "df -h");
    }
}
