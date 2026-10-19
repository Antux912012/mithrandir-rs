use std::io::Write;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use crate::ai::local_llm::{BuiltinLinuxAssistant, ChatMessage, LocalLLMManager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub success: bool,
    pub content: String,
    pub model: String,
    pub proposed_command: Option<String>,
}

pub struct AIClient {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl AIClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            base_url: Self::normalize_base_url(base_url),
            api_key: api_key.trim().to_string(),
            model: model.trim().to_string(),
        }
    }

    pub fn normalize_base_url(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.starts_with("builtin://") {
            return "builtin://linux-mini".to_string();
        }

        let mut url = trimmed.trim_end_matches('/').to_string();

        if url.contains(":generateContent") {
            if let Some(idx) = url.find("/models/") {
                url = url[..idx].trim_end_matches('/').to_string();
            }
        }

        for suffix in ["/chat/completions", "/completions", "/messages", "/models"] {
            if url.ends_with(suffix) {
                url = url.trim_end_matches(suffix).trim_end_matches('/').to_string();
            }
        }

        if url == "https://generativelanguage.googleapis.com" {
            return "https://generativelanguage.googleapis.com/v1beta".to_string();
        }
        if url == "https://api.anthropic.com" {
            return "https://api.anthropic.com/v1".to_string();
        }
        if url == "https://integrate.api.nvidia.com" {
            return "https://integrate.api.nvidia.com/v1".to_string();
        }
        if url == "http://localhost:11434" || url == "http://127.0.0.1:11434" {
            return "http://localhost:11434/v1".to_string();
        }
        if url == "http://localhost:1234" || url == "http://127.0.0.1:1234" {
            return "http://localhost:1234/v1".to_string();
        }
        if url == "http://localhost:18080" || url == "http://127.0.0.1:18080" {
            return "http://127.0.0.1:18080/v1".to_string();
        }

        if !url.ends_with("/v1")
            && !url.contains("/v1/")
            && !url.contains("/v1beta")
            && (url.starts_with("http://") || url.starts_with("https://"))
            && (url.contains("api.openai.com") || url.contains("nvidia.com") || url.contains("anthropic.com"))
        {
            url = format!("{}/v1", url);
        }

        url
    }

    pub fn is_builtin(&self) -> bool {
        self.base_url.starts_with("builtin://")
    }

    pub fn is_inapp_llamacpp(&self) -> bool {
        self.base_url.contains(":18080")
    }

    pub fn is_gemini(&self) -> bool {
        self.base_url.contains("generativelanguage.googleapis.com")
    }

    pub fn is_gemini_native(&self) -> bool {
        self.is_gemini() && !self.base_url.ends_with("/openai")
    }

    pub fn is_claude(&self) -> bool {
        self.base_url.contains("api.anthropic.com")
    }

    pub fn resolve_model(&self, model_override: Option<&str>) -> String {
        if let Some(m) = model_override {
            let trimmed = m.trim();
            if !trimmed.is_empty() {
                if self.is_gemini() {
                    return trimmed.trim_start_matches("models/").to_string();
                }
                return trimmed.to_string();
            }
        }
        if !self.model.trim().is_empty() {
            let trimmed = self.model.trim();
            if self.is_gemini() {
                return trimmed.trim_start_matches("models/").to_string();
            }
            return trimmed.to_string();
        }
        if self.is_builtin() {
            "builtin-linux-mini".to_string()
        } else if self.is_inapp_llamacpp() {
            "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string()
        } else if self.is_gemini() {
            "gemini-2.5-flash".to_string()
        } else if self.is_claude() {
            "claude-3-5-sonnet-latest".to_string()
        } else if self.base_url.contains("11434") {
            "qwen2.5:1.5b".to_string()
        } else if self.base_url.contains("nvidia.com") {
            "meta/llama-3.1-70b-instruct".to_string()
        } else {
            "gpt-4o-mini".to_string()
        }
    }

    pub fn build_gemini_payload(messages: &[ChatMessage], temperature: f64) -> serde_json::Value {
        let mut system_parts = Vec::new();
        let mut contents = Vec::new();

        for msg in messages {
            let role = msg.role.as_str();
            if role == "system" {
                if !msg.content.trim().is_empty() {
                    system_parts.push(serde_json::json!({ "text": msg.content }));
                }
            } else {
                let gemini_role = if role == "assistant" || role == "model" {
                    "model"
                } else {
                    "user"
                };
                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": [{ "text": msg.content }]
                }));
            }
        }

        if contents.is_empty() && !system_parts.is_empty() {
            contents.push(serde_json::json!({
                "role": "user",
                "parts": system_parts.clone()
            }));
            system_parts.clear();
        }

        let mut payload = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": temperature,
                "maxOutputTokens": 1024
            }
        });

        if !system_parts.is_empty() {
            payload["systemInstruction"] = serde_json::json!({
                "parts": system_parts
            });
        }

        payload
    }

    pub fn parse_gemini_response(resp_json: &serde_json::Value) -> Result<String, String> {
        if let Some(err_msg) = resp_json["error"]["message"].as_str() {
            return Err(format!("Gemini API Error: {}", err_msg));
        }

        if let Some(parts) = resp_json["candidates"][0]["content"]["parts"].as_array() {
            let mut text = String::new();
            for p in parts {
                if let Some(t) = p["text"].as_str() {
                    text.push_str(t);
                }
            }
            if !text.trim().is_empty() {
                return Ok(text.trim().to_string());
            }
        }

        Err("No content returned by Gemini API.".to_string())
    }

    pub fn test_connection(&self) -> Result<String, String> {
        if self.is_builtin() {
            return Ok("✓ Built-in offline Linux Assistant ready (Zero Download).".to_string());
        }

        if self.is_gemini_native() {
            if self.api_key.is_empty() {
                return Err("Google AI Studio API Key is required. Get one at aistudio.google.com.".to_string());
            }
            let models_url = format!("{}/models", self.base_url.trim_end_matches('/'));
            let mut cmd = Command::new("curl");
            cmd.args([
                "-s",
                "-f",
                "--max-time",
                "6",
                "-H",
                &format!("x-goog-api-key: {}", self.api_key),
                &models_url,
            ]);
            let output = cmd.output().map_err(|e| format!("Connection test failed: {}", e))?;
            if output.status.success() {
                return Ok(format!("✓ Connected to Google AI Studio Gemini API ({})", self.base_url));
            } else {
                return Err("Failed to authenticate with Google AI Studio. Check your Gemini API Key.".to_string());
            }
        }

        if self.is_claude() {
            if self.api_key.is_empty() {
                return Err("Anthropic Claude API Key is required.".to_string());
            }
            let models_url = format!("{}/models", self.base_url.trim_end_matches('/'));
            let mut cmd = Command::new("curl");
            cmd.args([
                "-s",
                "-f",
                "--max-time",
                "6",
                "-H",
                &format!("x-api-key: {}", self.api_key),
                "-H",
                "anthropic-version: 2023-06-01",
                &models_url,
            ]);
            let output = cmd.output().map_err(|e| format!("Connection test failed: {}", e))?;
            if output.status.success() {
                return Ok(format!("✓ Connected to Anthropic Claude API ({})", self.base_url));
            } else {
                return Err("Failed to authenticate with Anthropic Claude API. Check your API Key.".to_string());
            }
        }

        if self.is_inapp_llamacpp() && !LocalLLMManager::is_llamacpp_server_running() {
            let st = LocalLLMManager::check_llamacpp_status();
            if st.engine_installed && !st.models.is_empty() {
                let _ = LocalLLMManager::start_llamacpp_server(None);
            } else {
                return Err("In-app llama.cpp engine is not running yet. Click 'Install Qwen + llama.cpp' first.".to_string());
            }
        }

        let models_url = format!("{}/models", self.base_url.trim_end_matches('/'));
        let mut cmd = Command::new("curl");
        cmd.args(["-s", "-f", "--max-time", "6", &models_url]);
        if !self.api_key.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", self.api_key)]);
        }

        let output = cmd.output().map_err(|e| format!("Connection test failed: {}", e))?;
        if output.status.success() {
            Ok(format!("✓ Connection successful to {}", self.base_url))
        } else {
            // Fallback: check if root URL answers
            let root_url = self.base_url.trim_end_matches("/v1");
            let mut test_root = Command::new("curl");
            test_root.args(["-s", "-f", "--max-time", "3", root_url]);
            if let Ok(res) = test_root.output() {
                if res.status.success() {
                    return Ok(format!("✓ Endpoint responsive at {}", root_url));
                }
            }
            Err(format!("Endpoint failed to respond on {}", models_url))
        }
    }

    pub fn chat_completion(&self, messages: &[ChatMessage], model_override: Option<&str>) -> Result<ChatResponse, String> {
        if self.is_builtin() {
            let content = BuiltinLinuxAssistant::respond(messages);
            let proposed = Self::extract_proposed_command(&content);
            return Ok(ChatResponse {
                success: true,
                content,
                model: "builtin-linux-mini".to_string(),
                proposed_command: proposed,
            });
        }

        let target_model = self.resolve_model(model_override);

        if self.is_gemini_native() {
            if self.api_key.is_empty() {
                return Err("Google AI Studio API Key is required for Gemini.".to_string());
            }
            let clean_model = target_model.trim_start_matches("models/");
            let endpoint = format!(
                "{}/models/{}:generateContent",
                self.base_url.trim_end_matches('/'),
                clean_model
            );
            let body = Self::build_gemini_payload(messages, 0.3);
            let body_str = body.to_string();

            let mut cmd = Command::new("curl");
            cmd.args([
                "-s",
                "-X",
                "POST",
                "--max-time",
                "90",
                "-H",
                "Content-Type: application/json",
                "-H",
                "User-Agent: Mithrandir-RS/0.4.1",
                "-H",
                &format!("x-goog-api-key: {}", self.api_key),
                "--data-binary",
                "@-",
                &endpoint,
            ]);
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = cmd.spawn().map_err(|e| format!("Failed to call curl: {}", e))?;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(body_str.as_bytes());
            }

            let output = child.wait_with_output().map_err(|e| format!("HTTP request failed: {}", e))?;
            if !output.status.success() {
                let err_str = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Gemini API Network Error: {}", err_str));
            }

            let resp_json: serde_json::Value = serde_json::from_slice(&output.stdout)
                .map_err(|e| format!("Failed to parse Gemini JSON response: {}", e))?;

            let content = Self::parse_gemini_response(&resp_json)?;
            let proposed = Self::extract_proposed_command(&content);

            return Ok(ChatResponse {
                success: true,
                content,
                model: clean_model.to_string(),
                proposed_command: proposed,
            });
        }

        if self.is_claude() {
            if self.api_key.is_empty() {
                return Err("Anthropic Claude API Key is required.".to_string());
            }
            let endpoint = format!("{}/messages", self.base_url.trim_end_matches('/'));
            let mut system_parts = Vec::new();
            let mut chat_msgs = Vec::new();
            for m in messages {
                if m.role == "system" {
                    if !m.content.trim().is_empty() {
                        system_parts.push(m.content.clone());
                    }
                } else {
                    let r = if m.role == "assistant" { "assistant" } else { "user" };
                    chat_msgs.push(serde_json::json!({ "role": r, "content": m.content }));
                }
            }
            if chat_msgs.is_empty() && !system_parts.is_empty() {
                chat_msgs.push(serde_json::json!({ "role": "user", "content": system_parts.join("\n\n") }));
                system_parts.clear();
            }

            let mut body = serde_json::json!({
                "model": target_model,
                "messages": chat_msgs,
                "max_tokens": 1024,
                "temperature": 0.3
            });
            if !system_parts.is_empty() {
                body["system"] = serde_json::Value::String(system_parts.join("\n\n"));
            }

            let body_str = body.to_string();
            let mut cmd = Command::new("curl");
            cmd.args([
                "-s",
                "-X",
                "POST",
                "--max-time",
                "90",
                "-H",
                "Content-Type: application/json",
                "-H",
                "anthropic-version: 2023-06-01",
                "-H",
                &format!("x-api-key: {}", self.api_key),
                "--data-binary",
                "@-",
                &endpoint,
            ]);
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = cmd.spawn().map_err(|e| format!("Failed to call curl: {}", e))?;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(body_str.as_bytes());
            }

            let output = child.wait_with_output().map_err(|e| format!("HTTP request failed: {}", e))?;
            if !output.status.success() {
                let err_str = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Claude API Error: {}", err_str));
            }

            let resp_json: serde_json::Value = serde_json::from_slice(&output.stdout)
                .map_err(|e| format!("Failed to parse Claude JSON: {}", e))?;

            if let Some(err_msg) = resp_json["error"]["message"].as_str() {
                return Err(format!("Claude API Error: {}", err_msg));
            }

            let content = resp_json["content"][0]["text"]
                .as_str()
                .unwrap_or("No content received.")
                .to_string();
            let proposed = Self::extract_proposed_command(&content);

            return Ok(ChatResponse {
                success: true,
                content,
                model: target_model,
                proposed_command: proposed,
            });
        }

        if self.is_inapp_llamacpp() && !LocalLLMManager::is_llamacpp_server_running() {
            let _ = LocalLLMManager::start_llamacpp_server(None);
        }

        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": target_model,
            "messages": messages,
            "temperature": 0.3,
            "max_tokens": 1024
        });

        let body_str = body.to_string();

        let mut cmd = Command::new("curl");
        cmd.args([
            "-s", "-X", "POST",
            "--max-time", "90",
            "-H", "Content-Type: application/json",
            "-H", "User-Agent: Mithrandir-RS/0.4.1",
        ]);

        if !self.api_key.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", self.api_key)]);
        }

        cmd.args(["--data-binary", "@-", &endpoint]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| format!("Failed to call curl: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(body_str.as_bytes());
        }

        let output = child.wait_with_output().map_err(|e| format!("HTTP request failed: {}", e))?;
        if !output.status.success() {
            let err_str = String::from_utf8_lossy(&output.stderr);
            return Err(format!("AI Endpoint Error: {}", err_str));
        }

        let resp_json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

        if let Some(err_msg) = resp_json["error"]["message"].as_str() {
            return Err(format!("AI Endpoint Error: {}", err_msg));
        }

        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No content received.")
            .to_string();

        let proposed = Self::extract_proposed_command(&content);

        Ok(ChatResponse {
            success: true,
            content,
            model: target_model,
            proposed_command: proposed,
        })
    }

    pub fn extract_proposed_command(text: &str) -> Option<String> {
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(idx) = trimmed.find("PROPOSED_COMMAND:") {
                let cmd = trimmed[idx + 17..].trim();
                let clean = cmd.trim_matches('`').trim();
                if !clean.is_empty() {
                    return Some(clean.to_string());
                }
            }
        }
        None
    }
}
