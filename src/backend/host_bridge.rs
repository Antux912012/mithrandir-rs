use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

pub struct HostBridge;

impl HostBridge {
    /// Returns true if running within a Flatpak sandbox container.
    pub fn is_flatpak() -> bool {
        Path::new("/.flatpak-info").exists()
    }

    /// Wraps a command so it executes on the host OS if running inside Flatpak.
    pub fn wrap_command(cmd: &str, args: &[&str]) -> Command {
        if Self::is_flatpak() {
            let mut c = Command::new("flatpak-spawn");
            c.arg("--host");
            c.arg(cmd);
            c.args(args);
            c
        } else {
            let mut c = Command::new(cmd);
            c.args(args);
            c
        }
    }

    /// Resolves the absolute path of a binary on the host system.
    pub fn which(binary: &str) -> Option<String> {
        let mut cmd = if Self::is_flatpak() {
            let mut c = Command::new("flatpak-spawn");
            c.args(["--host", "which", binary]);
            c
        } else {
            let mut c = Command::new("which");
            c.arg(binary);
            c
        };

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Executes a command on the host and returns (exit_code, stdout, stderr).
    pub fn run_command(cmd: &str, args: &[&str]) -> Result<(i32, String, String), String> {
        let mut command = Self::wrap_command(cmd, args);
        let output = command
            .output()
            .map_err(|e| format!("Failed to execute '{}': {}", cmd, e))?;

        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((code, stdout, stderr))
    }

    /// Executes a command via Polkit pkexec for privilege elevation.
    pub fn run_privileged(args: &[&str]) -> Result<(i32, String, String), String> {
        Self::run_command("pkexec", args)
    }

    /// Spawns a process and streams merged stdout and stderr line-by-line into a callback.
    pub fn spawn_streaming<F>(
        cmd: &str,
        args: &[&str],
        privileged: bool,
        mut on_line: F,
    ) -> Result<i32, String>
    where
        F: FnMut(String),
    {
        let mut command = if privileged {
            let mut full_args = vec![cmd];
            full_args.extend_from_slice(args);
            Self::wrap_command("pkexec", &full_args)
        } else {
            Self::wrap_command(cmd, args)
        };

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{}': {}", cmd, e))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (tx, rx) = std::sync::mpsc::channel();
        let tx_err = tx.clone();

        let handle_out = std::thread::spawn(move || {
            if let Some(out) = stdout {
                let reader = BufReader::new(out);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx.send(line);
                }
            }
        });

        let handle_err = std::thread::spawn(move || {
            if let Some(err) = stderr {
                let reader = BufReader::new(err);
                for line in reader.lines().map_while(Result::ok) {
                    if !line.is_empty() {
                        let _ = tx_err.send(line);
                    }
                }
            }
        });

        // Consume merged lines in real-time as either pipe produces output
        for line in rx {
            on_line(line);
        }

        let _ = handle_out.join();
        let _ = handle_err.join();

        let status = child
            .wait()
            .map_err(|e| format!("Failed waiting for child process: {}", e))?;

        Ok(status.code().unwrap_or(-1))
    }
}
