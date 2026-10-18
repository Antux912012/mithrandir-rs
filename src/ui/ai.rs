use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, Entry, Frame, Label, Orientation, ProgressBar,
    ScrolledWindow, TextView, WrapMode,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, ComboRow, ExpanderRow, PasswordEntryRow, PreferencesGroup};
use std::sync::mpsc;
use crate::ai::{
    AIClient, ChatMessage, LocalLLMManager, SafetyGuard, get_recommended_qwen_models,
};
use crate::backend::diagnostics::DiagnosticsCollector;
use crate::backend::host_bridge::HostBridge;

pub struct AIPage;

impl AIPage {
    pub fn build() -> ScrolledWindow {
        let scroller = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .kinetic_scrolling(true)
            .build();

        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(20)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        // 1. In-App Offline Mini-LLM (Qwen 2.5 + llama.cpp) Group
        let offline_group = PreferencesGroup::builder()
            .title("Offline Assistant & Local Models")
            .description("Mithrandir ships with zero pre-bundled weights. You can run the built-in assistant offline with zero downloads, or install the recommended Qwen 2.5 Mini-LLM with llama.cpp to run locally inside Mithrandir.")
            .build();

        let llama_status_row = ActionRow::builder()
            .title("Mithrandir In-App Engine (Qwen 2.5 + llama.cpp)")
            .subtitle("Checking engine status...")
            .build();

        let install_qwen_btn = Button::builder()
            .label("Install Qwen + llama.cpp")
            .css_classes(vec!["suggested-action"])
            .valign(Align::Center)
            .build();

        let start_llama_btn = Button::builder()
            .label("Start")
            .valign(Align::Center)
            .build();

        let stop_llama_btn = Button::builder()
            .label("Stop")
            .valign(Align::Center)
            .build();

        let uninstall_llama_btn = Button::builder()
            .label("Uninstall")
            .css_classes(vec!["destructive-action"])
            .valign(Align::Center)
            .build();

        llama_status_row.add_suffix(&install_qwen_btn);
        llama_status_row.add_suffix(&start_llama_btn);
        llama_status_row.add_suffix(&stop_llama_btn);
        llama_status_row.add_suffix(&uninstall_llama_btn);
        offline_group.add(&llama_status_row);

        // Setup Console Expander
        let console_expander = ExpanderRow::builder()
            .title("Engine Setup & Download Console")
            .subtitle("View live download and runtime status logs")
            .build();

        let console_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let console_progress = ProgressBar::builder()
            .visible(false)
            .pulse_step(0.1)
            .build();

        let console_view = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(WrapMode::Word)
            .monospace(true)
            .height_request(120)
            .build();

        let console_scroll = ScrolledWindow::builder()
            .child(&console_view)
            .vexpand(true)
            .build();

        console_box.append(&console_progress);
        console_box.append(&console_scroll);
        console_expander.add_row(&ActionRow::builder().child(&console_box).build());
        offline_group.add(&console_expander);

        // Qwen GGUF Models Expander
        let qwen_models_expander = ExpanderRow::builder()
            .title("Recommended Qwen 2.5 GGUF Models")
            .subtitle("Curated high-efficiency models optimized for CPU inference on GNU/Linux")
            .build();

        for model in get_recommended_qwen_models() {
            let m_row = ActionRow::builder()
                .title(&model.name)
                .subtitle(&format!("{} | {} | {}", model.size, model.ram_required, model.description))
                .build();

            let dl_btn = Button::builder()
                .label("Download")
                .valign(Align::Center)
                .build();

            let m_id = model.id.clone();
            let console_view_clone = console_view.clone();
            let console_expander_clone = console_expander.clone();
            let console_progress_clone = console_progress.clone();
            let llama_status_row_clone = llama_status_row.clone();

            dl_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                console_expander_clone.set_expanded(true);
                console_progress_clone.set_visible(true);

                let (tx, rx) = mpsc::channel();
                let m_id_t = m_id.clone();

                std::thread::spawn(move || {
                    let tx_log = tx.clone();
                    let res = LocalLLMManager::download_gguf_model(
                        &m_id_t,
                        move |line| {
                            let _ = tx_log.send((false, line));
                        },
                        None,
                    );
                    let done_msg = match res {
                        Ok(msg) => msg,
                        Err(e) => format!("Download error: {}", e),
                    };
                    let _ = tx.send((true, done_msg));
                });

                let btn_c = btn.clone();
                let cv_c = console_view_clone.clone();
                let cp_c = console_progress_clone.clone();
                let sr_c = llama_status_row_clone.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    while let Ok((done, line)) = rx.try_recv() {
                        let buf = cv_c.buffer();
                        let mut end = buf.end_iter();
                        buf.insert(&mut end, &format!("{}\n", line));

                        if done {
                            btn_c.set_sensitive(true);
                            cp_c.set_visible(false);
                            let st = LocalLLMManager::check_llamacpp_status();
                            sr_c.set_subtitle(&format!("Running: {} | Models: {} | Disk: {:.1} MB", st.running, st.models.len(), st.disk_usage_mb));
                            return glib::ControlFlow::Break;
                        }
                    }
                    glib::ControlFlow::Continue
                });
            });

            m_row.add_suffix(&dl_btn);
            qwen_models_expander.add_row(&m_row);
        }
        offline_group.add(&qwen_models_expander);
        root_box.append(&offline_group);

        // 2. Provider & Endpoint Settings Group
        let provider_group = PreferencesGroup::builder()
            .title("Provider & Model Configuration")
            .description("Configure active LLM endpoint and model selection")
            .build();

        let provider_model = gtk4::StringList::new(&[
            "Mithrandir In-App Mini-LLM (Qwen 2.5 + llama.cpp — Offline)",
            "Built-in Linux Mini-Assistant (Offline, Zero-Download)",
            "Ollama (Local Offline — localhost:11434)",
            "LM Studio (Local Offline — localhost:1234)",
            "Google Gemini (Google AI Studio — Cloud API)",
            "Anthropic Claude (Cloud API)",
            "NVIDIA NIM (Cloud API)",
            "OpenAI / Custom OpenAI-Compatible Endpoint",
        ]);

        let provider_row = ComboRow::builder()
            .title("AI Provider")
            .subtitle("Select inference backend")
            .model(&provider_model)
            .selected(1) // default to Built-in Zero-Download
            .build();
        provider_group.add(&provider_row);

        let url_row = ActionRow::builder()
            .title("Endpoint URL")
            .subtitle("Base URL for Gemini AI Studio, Claude, OpenAI, or Local LLM API")
            .build();
        let url_entry = Entry::builder()
            .text("builtin://linux-mini")
            .hexpand(true)
            .valign(Align::Center)
            .build();
        url_row.add_suffix(&url_entry);
        provider_group.add(&url_row);

        let key_row = PasswordEntryRow::builder()
            .title("API Key (Google AI Studio / Claude / OpenAI / NIM — Optional for local)")
            .build();
        provider_group.add(&key_row);

        let model_row = ActionRow::builder()
            .title("Model Name")
            .subtitle("Model identifier passed to endpoint")
            .build();
        let model_entry = Entry::builder()
            .text("builtin-linux-mini")
            .hexpand(true)
            .valign(Align::Center)
            .build();
        model_row.add_suffix(&model_entry);
        provider_group.add(&model_row);

        let test_btn = Button::builder()
            .label("Test Connection")
            .valign(Align::Center)
            .build();
        let test_status_lbl = Label::builder()
            .label("")
            .xalign(0.0)
            .build();

        let test_row = ActionRow::builder()
            .title("Connection Diagnostic")
            .build();
        test_row.add_suffix(&test_btn);
        provider_group.add(&test_row);
        provider_group.add(&ActionRow::builder().child(&test_status_lbl).build());

        root_box.append(&provider_group);

        // 3. Generic Linux Task Assistant Group
        let task_group = PreferencesGroup::builder()
            .title("Interactive Linux Task Assistant")
            .description("Ask any system question or select a common diagnostic maintenance task")
            .build();

        let prompt_row = ActionRow::builder()
            .title("Command or Task Prompt")
            .build();

        let prompt_entry = Entry::builder()
            .placeholder_text("e.g. 'Find large files over 100MB' or 'Check open listening ports'")
            .hexpand(true)
            .valign(Align::Center)
            .build();

        let ask_btn = Button::builder()
            .label("Ask AI")
            .css_classes(vec!["suggested-action"])
            .valign(Align::Center)
            .build();

        prompt_row.add_suffix(&prompt_entry);
        prompt_row.add_suffix(&ask_btn);
        task_group.add(&prompt_row);

        // Quick task chips
        let chips_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .margin_top(4)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        let chips = [
            ("Find Large Files", "Find files over 100MB in my home directory"),
            ("Diagnose Memory", "Analyze high RAM and memory pressure"),
            ("Check Open Ports", "Check open network listening ports and sockets"),
            ("Fix Broken Packages", "Fix broken apt and dpkg package configuration"),
            ("Slow Boot Analysis", "Analyze slow boot time and systemd startup services"),
        ];

        for (label, query) in chips {
            let chip_btn = Button::builder()
                .label(label)
                .css_classes(vec!["pill"])
                .build();

            let pe_c = prompt_entry.clone();
            let q_str = query.to_string();
            chip_btn.connect_clicked(move |_| {
                pe_c.set_text(&q_str);
            });
            chips_box.append(&chip_btn);
        }

        task_group.add(&ActionRow::builder().child(&chips_box).build());

        // 4. System Log Triage
        let triage_row = ActionRow::builder()
            .title("1-Click System Log & Anomaly Triage")
            .subtitle("Collects anonymized OS, kernel, memory, failed services, and journal errors")
            .build();

        let triage_btn = Button::builder()
            .label("Gather Logs & Diagnose")
            .valign(Align::Center)
            .build();
        triage_row.add_suffix(&triage_btn);
        task_group.add(&triage_row);

        root_box.append(&task_group);

        // 5. Response & Proposed Command View Group
        let response_group = PreferencesGroup::builder()
            .title("Assistant Diagnosis & Proposed Command")
            .build();

        let response_view = TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(WrapMode::Word)
            .monospace(true)
            .height_request(150)
            .build();
        response_view.buffer().set_text("Ask a question above or click 'Gather Logs & Diagnose' to run system triage.");

        let frame = Frame::builder()
            .child(&response_view)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        response_group.add(&ActionRow::builder().child(&frame).build());

        let proposed_row = ActionRow::builder()
            .title("Proposed Terminal Fix")
            .subtitle("None proposed yet")
            .build();

        let exec_btn = Button::builder()
            .label("Execute Fix")
            .css_classes(vec!["suggested-action"])
            .valign(Align::Center)
            .sensitive(false)
            .build();

        proposed_row.add_suffix(&exec_btn);
        response_group.add(&proposed_row);

        root_box.append(&response_group);

        let clamp = Clamp::builder()
            .maximum_size(900)
            .child(&root_box)
            .build();
        scroller.set_child(Some(&clamp));

        // Sync Provider Combo with entries
        {
            let url_entry = url_entry.clone();
            let model_entry = model_entry.clone();

            provider_row.connect_selected_notify(move |combo| {
                match combo.selected() {
                    0 => {
                        url_entry.set_text("http://127.0.0.1:18080/v1");
                        model_entry.set_text("qwen2.5-1.5b-instruct-q4_k_m.gguf");
                    }
                    1 => {
                        url_entry.set_text("builtin://linux-mini");
                        model_entry.set_text("builtin-linux-mini");
                    }
                    2 => {
                        url_entry.set_text("http://localhost:11434/v1");
                        model_entry.set_text("qwen2.5:1.5b");
                    }
                    3 => {
                        url_entry.set_text("http://localhost:1234/v1");
                        model_entry.set_text("local-model");
                    }
                    4 => {
                        url_entry.set_text("https://generativelanguage.googleapis.com/v1beta");
                        model_entry.set_text("gemini-2.5-flash");
                    }
                    5 => {
                        url_entry.set_text("https://api.anthropic.com/v1");
                        model_entry.set_text("claude-3-5-sonnet-latest");
                    }
                    6 => {
                        url_entry.set_text("https://integrate.api.nvidia.com/v1");
                        model_entry.set_text("meta/llama-3.1-70b-instruct");
                    }
                    7 => {
                        url_entry.set_text("https://api.openai.com/v1");
                        model_entry.set_text("gpt-4o-mini");
                    }
                    _ => {}
                }
            });
        }

        // Live Llama Status Refresh Loop
        {
            let llama_status_row = llama_status_row.clone();
            let start_llama_btn = start_llama_btn.clone();
            let stop_llama_btn = stop_llama_btn.clone();

            let update_status = move || {
                let st = LocalLLMManager::check_llamacpp_status();
                if st.running {
                    llama_status_row.set_subtitle(&format!(
                        "Active & Running on {} (Model: {}) | Disk: {:.1} MB",
                        st.base_url, st.active_model, st.disk_usage_mb
                    ));
                    start_llama_btn.set_sensitive(false);
                    stop_llama_btn.set_sensitive(true);
                } else if st.engine_installed && !st.models.is_empty() {
                    llama_status_row.set_subtitle(&format!(
                        "Installed & Stopped | {} models available | Disk: {:.1} MB",
                        st.models.len(), st.disk_usage_mb
                    ));
                    start_llama_btn.set_sensitive(true);
                    stop_llama_btn.set_sensitive(false);
                } else {
                    llama_status_row.set_subtitle("Not installed yet. Click 'Install Qwen + llama.cpp' to setup.");
                    start_llama_btn.set_sensitive(false);
                    stop_llama_btn.set_sensitive(false);
                }
                glib::ControlFlow::Continue
            };

            update_status();
            glib::timeout_add_seconds_local(3, update_status);
        }

        // Connect In-App Engine Buttons (Install, Start, Stop, Uninstall)
        {
            let cv = console_view.clone();
            let ce = console_expander.clone();
            let cp = console_progress.clone();
            let sr = llama_status_row.clone();

            install_qwen_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                ce.set_expanded(true);
                cp.set_visible(true);

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let tx_log = tx.clone();
                    let res = LocalLLMManager::install_qwen_with_llamacpp(
                        None,
                        move |line| {
                            let _ = tx_log.send((false, line));
                        },
                        None,
                    );
                    let done_msg = match res {
                        Ok(msg) => msg,
                        Err(e) => format!("Installation error: {}", e),
                    };
                    let _ = tx.send((true, done_msg));
                });

                let btn_c = btn.clone();
                let cv_c = cv.clone();
                let cp_c = cp.clone();
                let sr_c = sr.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    while let Ok((done, line)) = rx.try_recv() {
                        let buf = cv_c.buffer();
                        let mut end = buf.end_iter();
                        buf.insert(&mut end, &format!("{}\n", line));

                        if done {
                            btn_c.set_sensitive(true);
                            cp_c.set_visible(false);
                            let st = LocalLLMManager::check_llamacpp_status();
                            sr_c.set_subtitle(&format!("Running: {} | Disk: {:.1} MB", st.running, st.disk_usage_mb));
                            return glib::ControlFlow::Break;
                        }
                    }
                    glib::ControlFlow::Continue
                });
            });
        }

        {
            let sr = llama_status_row.clone();
            start_llama_btn.connect_clicked(move |_| {
                let _ = LocalLLMManager::start_llamacpp_server(None);
                let st = LocalLLMManager::check_llamacpp_status();
                sr.set_subtitle(&format!("Running: {} | Model: {}", st.running, st.active_model));
            });

            let sr_stop = llama_status_row.clone();
            stop_llama_btn.connect_clicked(move |_| {
                let _ = LocalLLMManager::stop_llamacpp_server();
                let st = LocalLLMManager::check_llamacpp_status();
                sr_stop.set_subtitle(&format!("Running: {} (Stopped)", st.running));
            });

            let sr_un = llama_status_row.clone();
            uninstall_llama_btn.connect_clicked(move |_| {
                let _ = LocalLLMManager::uninstall_llamacpp_and_models(true, true);
                let st = LocalLLMManager::check_llamacpp_status();
                sr_un.set_subtitle(&format!("Uninstalled. Running: {} | Models: {}", st.running, st.models.len()));
            });
        }

        // Test Connection Button
        {
            let url_entry = url_entry.clone();
            let key_row = key_row.clone();
            let model_entry = model_entry.clone();
            let test_status_lbl = test_status_lbl.clone();

            test_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                test_status_lbl.set_label("Testing connection...");

                let client = AIClient::new(
                    &url_entry.text(),
                    &key_row.text(),
                    &model_entry.text(),
                );

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let res = client.test_connection();
                    let _ = tx.send(res);
                });

                let btn_c = btn.clone();
                let lbl_c = test_status_lbl.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if let Ok(res) = rx.try_recv() {
                        btn_c.set_sensitive(true);
                        match res {
                            Ok(msg) => lbl_c.set_label(&msg),
                            Err(e) => lbl_c.set_label(&format!("⚠️ Error: {}", e)),
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            });
        }

        // Query Helper Closure
        let send_query = {
            let url_entry = url_entry.clone();
            let key_row = key_row.clone();
            let model_entry = model_entry.clone();
            let response_view = response_view.clone();
            let proposed_row = proposed_row.clone();
            let exec_btn = exec_btn.clone();

            move |prompt: String| {
                let client = AIClient::new(
                    &url_entry.text(),
                    &key_row.text(),
                    &model_entry.text(),
                );

                let rv = response_view.clone();
                let pr = proposed_row.clone();
                let eb = exec_btn.clone();

                rv.buffer().set_text("Sending query to assistant...");
                eb.set_sensitive(false);
                pr.set_subtitle("Thinking...");

                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let msgs = vec![ChatMessage {
                        role: "user".to_string(),
                        content: prompt,
                    }];
                    let res = client.chat_completion(&msgs, None);
                    let _ = tx.send(res);
                });

                glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
                    if let Ok(res) = rx.try_recv() {
                        match res {
                            Ok(resp) => {
                                rv.buffer().set_text(&resp.content);
                                if let Some(cmd) = resp.proposed_command {
                                    let (safe, reason) = SafetyGuard::is_command_safe(&cmd);
                                    if safe {
                                        pr.set_subtitle(&format!("PROPOSED: {}", cmd));
                                        eb.set_sensitive(true);
                                    } else {
                                        pr.set_subtitle(&format!("PROPOSED: {} (Blocked: {})", cmd, reason));
                                        eb.set_sensitive(false);
                                    }
                                } else {
                                    pr.set_subtitle("No concrete command proposed.");
                                    eb.set_sensitive(false);
                                }
                            }
                            Err(e) => {
                                rv.buffer().set_text(&format!("Assistant Error: {}", e));
                                pr.set_subtitle("Error occurred.");
                                eb.set_sensitive(false);
                            }
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            }
        };

        // Ask Button
        {
            let pe = prompt_entry.clone();
            let sq = send_query.clone();
            ask_btn.connect_clicked(move |_| {
                let text = pe.text().trim().to_string();
                if !text.is_empty() {
                    sq(text);
                }
            });
        }

        // Triage Button
        {
            let sq = send_query.clone();
            triage_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let snapshot = DiagnosticsCollector::gather_system_snapshot();
                    let _ = tx.send(snapshot);
                });

                let btn_c = btn.clone();
                let sq_c = sq.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if let Ok(snapshot) = rx.try_recv() {
                        btn_c.set_sensitive(true);
                        sq_c(snapshot);
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            });
        }

        // Execute Fix Button
        {
            let proposed_row = proposed_row.clone();
            let response_view = response_view.clone();

            exec_btn.connect_clicked(move |btn| {
                let sub = proposed_row.subtitle().unwrap_or_default().to_string();
                if let Some(cmd) = sub.strip_prefix("PROPOSED: ") {
                    btn.set_sensitive(false);
                    let cmd_str = cmd.trim().to_string();
                    let cmd_exec = cmd_str.clone();
                    let rv = response_view.clone();
                    let btn_c = btn.clone();

                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let res = HostBridge::run_command("sh", &["-c", &cmd_exec]);
                        let _ = tx.send(res);
                    });

                    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                        if let Ok(res) = rx.try_recv() {
                            btn_c.set_sensitive(true);
                            let buf = rv.buffer();
                            let mut end = buf.end_iter();
                            match res {
                                Ok((code, stdout, stderr)) => {
                                    buf.insert(&mut end, &format!("\n\n--- Executed: {} (Exit Code: {}) ---\nSTDOUT:\n{}\nSTDERR:\n{}", cmd_str, code, stdout, stderr));
                                }
                                Err(e) => {
                                    buf.insert(&mut end, &format!("\n\nExecution failed: {}", e));
                                }
                            }
                            glib::ControlFlow::Break
                        } else {
                            glib::ControlFlow::Continue
                        }
                    });
                }
            });
        }

        scroller
    }
}
