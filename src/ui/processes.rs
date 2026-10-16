use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, ListBox, Orientation, ScrolledWindow, SearchEntry,
};
use libadwaita as adw;
use adw::prelude::*;
use adw::{ActionRow, Clamp, PreferencesGroup};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use crate::backend::processes::{ProcessItem, ProcessManager};

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortOrder {
    Ram,
    Cpu,
    Name,
}

pub struct ProcessesPage;

impl ProcessesPage {
    pub fn build() -> ScrolledWindow {
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .build();

        // Header controls
        let header_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .build();

        let search_entry = SearchEntry::builder()
            .placeholder_text("Search processes by name or PID...")
            .hexpand(true)
            .build();

        let sort_ram_btn = Button::builder()
            .label("RAM ↓")
            .tooltip_text("Sort by memory usage")
            .build();

        let sort_cpu_btn = Button::builder()
            .label("CPU ↓")
            .tooltip_text("Sort by processor load")
            .build();

        let sort_name_btn = Button::builder()
            .label("Name")
            .tooltip_text("Sort alphabetically")
            .build();

        let refresh_btn = Button::builder()
            .icon_name("view-refresh-symbolic")
            .tooltip_text("Refresh process list")
            .build();

        header_box.append(&search_entry);
        header_box.append(&sort_ram_btn);
        header_box.append(&sort_cpu_btn);
        header_box.append(&sort_name_btn);
        header_box.append(&refresh_btn);
        root_box.append(&header_box);

        // Group & List
        let group = PreferencesGroup::builder()
            .title("Active Processes")
            .description("Real-time task monitor and memory usage")
            .build();

        let list_box = ListBox::builder()
            .css_classes(vec!["boxed-list"])
            .selection_mode(gtk4::SelectionMode::None)
            .build();
        group.add(&list_box);
        root_box.append(&group);

        let clamp = Clamp::builder()
            .maximum_size(960)
            .child(&root_box)
            .build();
        scrolled.set_child(Some(&clamp));

        let all_procs: Rc<RefCell<Vec<ProcessItem>>> = Rc::new(RefCell::new(Vec::new()));
        let filter_text: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
        let current_sort: Rc<RefCell<SortOrder>> = Rc::new(RefCell::new(SortOrder::Ram));

        let populate_list = {
            let list_box = list_box.clone();
            let all_procs = all_procs.clone();
            let filter_text = filter_text.clone();
            let current_sort = current_sort.clone();

            move || {
                // Clear existing
                while let Some(child) = list_box.first_child() {
                    list_box.remove(&child);
                }

                let query = filter_text.borrow().to_lowercase();
                let mut procs = all_procs.borrow().clone();

                match *current_sort.borrow() {
                    SortOrder::Ram => {
                        procs.sort_by(|a, b| b.mem_mb.partial_cmp(&a.mem_mb).unwrap_or(std::cmp::Ordering::Equal));
                    }
                    SortOrder::Cpu => {
                        procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
                    }
                    SortOrder::Name => {
                        procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    }
                }

                let mut count = 0;
                for p in procs.iter() {
                    if !query.is_empty() {
                        let name_match = p.name.to_lowercase().contains(&query);
                        let pid_match = p.pid.to_string().contains(&query);
                        if !name_match && !pid_match {
                            continue;
                        }
                    }

                    if count >= 100 {
                        break; // Display top 100 to maintain smooth 60fps GUI
                    }
                    count += 1;

                    let row = ActionRow::builder()
                        .title(&format!("{} (PID: {})", p.name, p.pid))
                        .subtitle(&format!("CPU: {:.1}% | RAM: {:.1} MB ({:.1}%)", p.cpu_percent, p.mem_mb, p.mem_percent))
                        .build();

                    let kill_btn = Button::builder()
                        .label("End")
                        .css_classes(vec!["destructive-action"])
                        .valign(Align::Center)
                        .build();

                    let pid = p.pid;
                    let list_box_inner = list_box.clone();
                    let row_inner = row.clone();

                    kill_btn.connect_clicked(move |b| {
                        b.set_sensitive(false);
                        let _ = ProcessManager::kill_process(pid, false);
                        list_box_inner.remove(&row_inner);
                    });

                    row.add_suffix(&kill_btn);
                    list_box.append(&row);
                }
            }
        };

        // Async refresh function
        let refresh_list = {
            let all_procs = all_procs.clone();
            let populate_list = populate_list.clone();

            move || {
                let (tx, rx) = mpsc::channel();
                std::thread::spawn(move || {
                    let procs = ProcessManager::list_processes();
                    let _ = tx.send(procs);
                });

                let all_procs_inner = all_procs.clone();
                let populate_inner = populate_list.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                    if let Ok(procs) = rx.try_recv() {
                        *all_procs_inner.borrow_mut() = procs;
                        populate_inner();
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
            }
        };

        // Connect sort buttons
        {
            let cs = current_sort.clone();
            let pl = populate_list.clone();
            sort_ram_btn.connect_clicked(move |_| {
                *cs.borrow_mut() = SortOrder::Ram;
                pl();
            });

            let cs2 = current_sort.clone();
            let pl2 = populate_list.clone();
            sort_cpu_btn.connect_clicked(move |_| {
                *cs2.borrow_mut() = SortOrder::Cpu;
                pl2();
            });

            let cs3 = current_sort.clone();
            let pl3 = populate_list.clone();
            sort_name_btn.connect_clicked(move |_| {
                *cs3.borrow_mut() = SortOrder::Name;
                pl3();
            });
        }

        // Connect search changed
        let filter_clone = filter_text.clone();
        let populate_clone = populate_list.clone();
        search_entry.connect_search_changed(move |entry| {
            *filter_clone.borrow_mut() = entry.text().to_string();
            populate_clone();
        });

        // Connect refresh button
        let refresh_clone = refresh_list.clone();
        refresh_btn.connect_clicked(move |_| {
            refresh_clone();
        });

        // Initial load
        refresh_list();

        scrolled
    }
}
