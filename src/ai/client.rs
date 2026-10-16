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
        if url.ends_with("/chat/completions") {
            url = url.trim_end_matches("/chat/completions").to_string();
        }
        if url.ends_with("/models") {
            url = url.trim_end_matches("/models").to_string();
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

        if !url.ends_with("/v1") && !url.contains("/v1/") && (url.starts_with("http://") || url.starts_with("https://")) {
            if url.contains("api.openai.com") || url.contains("nvidia.com") {
                url = format!("{}/v1", url);
            }
        }

        url
    }

    pub fn is_builtin(&self) -> bool {
        self.base_url.starts_with("builtin://")
    }

    pub fn is_inapp_llamacpp(&self) -> bool {
        self.base_url.contains(":18080")
    }

    pub fn resolve_model(&self, model_override: Option<&str>) -> String {
        if let Some(m) = model_override {
            if !m.trim().is_empty() {
                return m.trim().to_string();
            }
        }
        if !self.model.trim().is_empty() {
            return self.model.trim().to_string();
        }
        if self.is_builtin() {
            "builtin-linux-mini".to_string()
        } else if self.is_inapp_llamacpp() {
            "qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string()
        } else if self.base_url.contains("11434") {
            "qwen2.5:1.5b".to_string()
        } else if self.base_url.contains("nvidia.com") {
            "meta/llama-3.1-70b-instruct".to_string()
        } else {
            "gpt-4o-mini".to_string()
        }
    }

    pub fn test_connection(&self) -> Result<String, String> {
        if self.is_builtin() {
            return Ok("✓ Built-in offline Linux Assistant ready (Zero Download).".to_string());
        }

        if self.is_inapp_llamacpp() {
            if !LocalLLMManager::is_llamacpp_server_running() {
                let st = LocalLLMManager::check_llamacpp_status();
                if st.engine_installed && !st.models.is_empty() {
                    let _ = LocalLLMManager::start_llamacpp_server(None);
                } else {
                    return Err("In-app llama.cpp engine is not running yet. Click 'Install Qwen + llama.cpp' first.".to_string());
                }
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

        if self.is_inapp_llamacpp() && !LocalLLMManager::is_llamacpp_server_running() {
            let _ = LocalLLMManager::start_llamacpp_server(None);
        }

        let target_model = self.resolve_model(model_override);
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
            "-H", "User-Agent: Mithrandir-RS/0.4.0",
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
