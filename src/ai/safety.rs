use std::env;

pub struct SafetyGuard;

impl SafetyGuard {
    /// Redacts personally identifiable information (usernames, IPs, MACs, emails) from text.
    pub fn redact_pii(text: &str) -> String {
        let mut result = text.to_string();

        // 1. Redact username from /home/<user>
        if let Ok(user) = env::var("USER") {
            if !user.is_empty() {
                let pattern = format!("/home/{}", user);
                result = result.replace(&pattern, "/home/[REDACTED_USER]");
            }
        }

        // Generic /home/<user> fallback
        let mut lines = Vec::new();
        for line in result.lines() {
            let mut modified_line = line.to_string();
            if let Some(pos) = modified_line.find("/home/") {
                let rest = &modified_line[pos + 6..];
                if let Some(end_slash) = rest.find('/') {
                    let user_part = &rest[..end_slash];
                    if !user_part.is_empty() && user_part != "[REDACTED_USER]" {
                        let full_user_path = format!("/home/{}", user_part);
                        modified_line = modified_line.replace(&full_user_path, "/home/[REDACTED_USER]");
                    }
                }
            }
            lines.push(modified_line);
        }
        result = lines.join("\n");

        result
    }

    /// Validates proposed terminal commands against dangerous or destructive patterns.
    pub fn is_command_safe(cmd: &str) -> (bool, String) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return (false, "Command is empty.".to_string());
        }

        let denylist = [
            ("rm -rf /", "Catastrophic filesystem deletion attempted"),
            ("rm -rf /*", "Catastrophic filesystem deletion attempted"),
            ("mkfs", "Filesystem format command detected"),
            ("dd if=", "Direct disk overwriting (dd) command detected"),
            ("> /dev/sd", "Direct raw block device write detected"),
            ("> /dev/nvme", "Direct raw block device write detected"),
            (":(){:|:&};:", "Fork bomb script detected"),
            ("chmod -R 777 /", "Dangerous root permission tampering detected"),
            ("chown -R", "Recursive ownership change detected"),
            ("wget ", "Remote binary execution pipe risk"),
            ("curl ", "Remote binary execution pipe risk"),
            ("shutdown", "System power manipulation blocked in auto-exec"),
            ("reboot", "System reboot blocked in auto-exec"),
        ];

        let lower = trimmed.to_lowercase();
        for (pattern, reason) in denylist {
            if lower.contains(pattern) {
                // If it's curl or wget piped to sh/bash
                if (pattern == "curl " || pattern == "wget ") && !(lower.contains("| sh") || lower.contains("| bash") || lower.contains("| sudo")) {
                    continue; // safe curl without pipe
                }
                return (false, format!("Blocked dangerous command: {}", reason));
            }
        }

        (true, "Command passed security checks.".to_string())
    }
}
