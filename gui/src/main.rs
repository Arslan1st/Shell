use eframe::egui;
use std::{collections::HashSet, fs, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind {
    Item,
    Menu,
    Separator,
}

#[derive(Clone)]
struct Entry {
    kind: EntryKind,
    title: String,
    command: String,
    icon: String,
    enabled: bool,
    depth: usize,
}

impl Entry {
    fn label(&self) -> String {
        match self.kind {
            EntryKind::Separator => "Separator".into(),
            _ if self.title.is_empty() => "Untitled entry".into(),
            _ => self.title.clone(),
        }
    }

    fn to_nss(&self, indent: &str) -> String {
        match self.kind {
            EntryKind::Separator => format!("{indent}separator\n"),
            EntryKind::Item => format!(
                "{indent}item(title = '{}' command = '{}' image = '{}' enabled = {})\n",
                esc(&self.title),
                esc(&self.command),
                esc(&self.icon),
                self.enabled as u8
            ),
            EntryKind::Menu => {
                format!(
                    "{indent}menu(title = '{}' image = '{}') {{\n",
                    esc(&self.title),
                    esc(&self.icon)
                )
            }
        }
    }
}

fn esc(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

struct App {
    entries: Vec<Entry>,
    selected: Option<usize>,
    expanded: HashSet<usize>,
    file_path: String,
    status: String,
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            entries: Vec::new(),
            selected: None,
            expanded: HashSet::new(),
            file_path: "../src/bin/imports/gui.nss".into(),
            status: "Ready".into(),
        };
        app.load();
        app
    }
}

impl App {
    fn selected_mut(&mut self) -> Option<&mut Entry> {
        self.selected.and_then(|i| self.entries.get_mut(i))
    }

    // Insert under the selected menu, or after the selected entry's entire subtree.
    fn add_entry(&mut self, kind: EntryKind) {
        let (insert_at, depth) = match self.selected {
            Some(index) if index < self.entries.len() => {
                let selected = &self.entries[index];
                if selected.kind == EntryKind::Menu {
                    let depth = selected.depth + 1;
                    self.expanded.insert(index);
                    (index + 1, depth)
                } else {
                    let depth = selected.depth;
                    let mut at = index + 1;
                    while at < self.entries.len() && self.entries[at].depth > depth {
                        at += 1;
                    }
                    (at, depth)
                }
            }
            _ => (self.entries.len(), 0),
        };

        self.expanded = self
            .expanded
            .iter()
            .map(|&index| if index >= insert_at { index + 1 } else { index })
            .collect();

        let title = match kind {
            EntryKind::Item => "New item",
            EntryKind::Menu => "New menu",
            EntryKind::Separator => "",
        };

        self.entries.insert(
            insert_at,
            Entry {
                kind,
                title: title.into(),
                command: String::new(),
                icon: String::new(),
                enabled: true,
                depth,
            },
        );
        self.selected = Some(insert_at);
    }

    fn remove_entry(&mut self, index: usize) {
        if index >= self.entries.len() {
            return;
        }

        // Removing a menu also removes every descendant belonging to that menu.
        let depth = self.entries[index].depth;
        let end = if self.entries[index].kind == EntryKind::Menu {
            let mut end = index + 1;
            while end < self.entries.len() && self.entries[end].depth > depth {
                end += 1;
            }
            end
        } else {
            index + 1
        };

        let count = end - index;
        self.entries.drain(index..end);

        self.expanded = self
            .expanded
            .iter()
            .filter_map(|&expanded_index| {
                if expanded_index >= index && expanded_index < end {
                    None
                } else if expanded_index >= end {
                    Some(expanded_index - count)
                } else {
                    Some(expanded_index)
                }
            })
            .collect();

        self.selected = if self.entries.is_empty() {
            None
        } else {
            Some(index.min(self.entries.len() - 1))
        };
    }

    fn load(&mut self) {
        match fs::read_to_string(PathBuf::from(&self.file_path)) {
            Ok(contents) => {
                self.entries = parse_nss(&contents);
                self.expanded = self
                    .entries
                    .iter()
                    .enumerate()
                    .filter(|(_, entry)| entry.kind == EntryKind::Menu)
                    .map(|(i, _)| i)
                    .collect();
                self.selected = if self.entries.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.status = format!("Loaded {} entries", self.entries.len());
            }
            Err(e) => self.status = format!("Load failed: {e}"),
        }
    }

    fn save(&mut self) {
        let mut contents = String::new();

        for (i, entry) in self.entries.iter().enumerate() {
            let indent = "    ".repeat(entry.depth);
            contents.push_str(&entry.to_nss(&indent));

            let next_depth = self.entries.get(i + 1).map_or(0, |next| next.depth);
            if entry.kind == EntryKind::Menu && next_depth <= entry.depth {
                contents.push_str(&format!("{}}}\n", indent));
            }
        }

        let max_depth = self
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Menu)
            .map(|entry| entry.depth)
            .max();

        if let Some(depth) = max_depth {
            contents.push_str(&"}\n".repeat(depth + 1));
        }

        match fs::write(PathBuf::from(&self.file_path), contents) {
            Ok(_) => self.status = format!("Saved {} entries", self.entries.len()),
            Err(e) => self.status = format!("Save failed: {e}"),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Shell Context Menu");
                ui.separator();
                ui.label("Rust editor");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save changes").clicked() {
                        self.save();
                    }
                });
            });
        });

        // Fixed header/buttons; only the entry tree below the separator scrolls.
        egui::SidePanel::left("entries")
            .resizable(true)
            .default_width(280.0)
            .width_range(220.0..=500.0)
            .show(ctx, |ui| {
                ui.heading("Menu entries");
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    if ui.button("+ Item").clicked() {
                        self.add_entry(EntryKind::Item);
                    }
                    if ui.button("+ Menu").clicked() {
                        self.add_entry(EntryKind::Menu);
                    }
                    if ui.button("+ Separator").clicked() {
                        self.add_entry(EntryKind::Separator);
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("menu_entry_tree")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut remove = None;
                        let mut toggle = None;
                        let mut collapsed_depths: Vec<usize> = Vec::new();

                        for (i, entry) in self.entries.iter().enumerate() {
                            // Pop collapsed ancestors when we reach a sibling or ancestor level.
                            while let Some(&collapsed_depth) = collapsed_depths.last() {
                                if entry.depth <= collapsed_depth {
                                    collapsed_depths.pop();
                                } else {
                                    break;
                                }
                            }

                            // If any ancestor is collapsed, this entry is hidden.
                            if !collapsed_depths.is_empty() {
                                continue;
                            }

                            let selected = self.selected == Some(i);
                            let is_menu = entry.kind == EntryKind::Menu;
                            let is_expanded = self.expanded.contains(&i);
                            let marker = match entry.kind {
                                EntryKind::Menu if is_expanded => "-",
                                EntryKind::Menu => "+",
                                EntryKind::Item => " ",
                                EntryKind::Separator => " ",
                            };
                            let icon = if entry.icon.trim().is_empty() {
                                "□"
                            } else {
                                "◆"
                            };
                            let label = format!("{icon}  {}", entry.label());
                            let mut row_toggle = false;

                            ui.horizontal(|ui| {
                                ui.add_space(entry.depth as f32 * 16.0);

                                if is_menu {
                                    if ui.small_button(marker).clicked() {
                                        row_toggle = true;
                                    }
                                } else {
                                    ui.add_space(22.0);
                                }

                                if ui.selectable_label(selected, label).clicked() {
                                    self.selected = Some(i);
                                }

                                if selected && ui.small_button("×").clicked() {
                                    remove = Some(i);
                                }
                            });

                            if row_toggle {
                                toggle = Some(i);
                            }

                            // Don't push a menu if it is already hidden by a collapsed ancestor;
                            // hidden descendants are skipped above.
                            if is_menu && !is_expanded {
                                collapsed_depths.push(entry.depth);
                            }
                        }

                        // Mutate only after iteration to avoid borrowing conflicts.
                        if let Some(i) = toggle {
                            if !self.expanded.remove(&i) {
                                self.expanded.insert(i);
                            }
                        }

                        if let Some(i) = remove {
                            self.remove_entry(i);
                        }
                    });
            });

        // Live, visual approximation of the resulting context menu. The preview is
        // rebuilt every frame, so changes in the properties editor appear immediately.
        egui::SidePanel::right("live_context_preview")
            .resizable(true)
            .default_width(300.0)
            .width_range(240.0..=420.0)
            .show(ctx, |ui| {
                ui.heading("Live preview");
                ui.label(
                    egui::RichText::new("Updates as you edit entries")
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(8.0);

                ui.label("Context menu");
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_min_width(ui.available_width().max(180.0));
                    let mut has_root_entry = false;

                    for (index, entry) in self.entries.iter().enumerate() {
                        if entry.depth != 0 {
                            continue;
                        }
                        has_root_entry = true;
                        show_preview_row(ui, entry, self.selected == Some(index));
                    }

                    if !has_root_entry {
                        ui.label(
                            egui::RichText::new("Add a top-level item or menu to see the preview.")
                                .italics()
                                .color(egui::Color32::GRAY),
                        );
                    }
                });

                // Show a submenu preview for the currently selected menu, or the first
                // top-level menu with children when no menu is selected.
                let preview_menu_index = self
                    .selected
                    .and_then(|selected_index| {
                        let selected_entry = self.entries.get(selected_index)?;

                        // If an entry inside a submenu is selected, preview its nearest
                        // containing menu. If the selected entry is itself a menu, use it.
                        if selected_entry.kind == EntryKind::Menu {
                            return Some(selected_index);
                        }

                        let selected_depth = selected_entry.depth;
                        self.entries[..selected_index]
                            .iter()
                            .enumerate()
                            .rev()
                            .find(|(_, candidate)| {
                                candidate.kind == EntryKind::Menu
                                    && candidate.depth < selected_depth
                            })
                            .map(|(index, _)| index)
                    })
                    .or_else(|| {
                        self.entries.iter().enumerate().find_map(|(i, entry)| {
                            if entry.kind != EntryKind::Menu {
                                return None;
                            }
                            let depth = entry.depth;
                            self.entries
                                .get(i + 1)
                                .filter(|next| next.depth > depth)
                                .map(|_| i)
                        })
                    });

                if let Some(menu_index) = preview_menu_index {
                    let menu = &self.entries[menu_index];
                    let menu_title = menu.label();
                    let menu_depth = menu.depth;
                    let has_children = self
                        .entries
                        .get(menu_index + 1)
                        .is_some_and(|next| next.depth > menu_depth);

                    if has_children {
                        ui.add_space(12.0);
                        ui.label(format!("Submenu: {menu_title}"));
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_min_width(ui.available_width().max(180.0));
                            for (child_index, child) in
                                self.entries.iter().enumerate().skip(menu_index + 1)
                            {
                                if child.depth <= menu_depth {
                                    break;
                                }
                                // This popup shows only direct children, like one submenu level.
                                if child.depth != menu_depth + 1 {
                                    continue;
                                }
                                show_preview_row(ui, child, self.selected == Some(child_index));
                            }
                        });
                    }
                }

                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(
                        "This is an in-app visual preview; actual Windows styling may differ.",
                    )
                    .small()
                    .color(egui::Color32::GRAY),
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("Entry properties");
            ui.add_space(12.0);

            if let Some(entry) = self.selected_mut() {
                ui.horizontal(|ui| {
                    ui.label("Type");
                    ui.radio_value(&mut entry.kind, EntryKind::Item, "Item");
                    ui.radio_value(&mut entry.kind, EntryKind::Menu, "Menu");
                    ui.radio_value(&mut entry.kind, EntryKind::Separator, "Separator");
                });

                ui.add_space(14.0);

                if entry.kind != EntryKind::Separator {
                    ui.label("Title");
                    ui.text_edit_singleline(&mut entry.title);
                    ui.add_space(8.0);
                    ui.label("Icon / glyph");
                    ui.text_edit_singleline(&mut entry.icon);
                }

                if entry.kind == EntryKind::Item {
                    ui.add_space(8.0);
                    ui.label("Command");
                    ui.add(
                        egui::TextEdit::multiline(&mut entry.command)
                            .desired_rows(4)
                            .hint_text("Example: wt.exe -d %sel.path"),
                    );
                    ui.checkbox(&mut entry.enabled, "Enabled");
                }

                ui.separator();
                ui.label("Preview");
                ui.code(entry.to_nss(""));
            } else {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "Select an entry or add one from the left.",
                );
            }

            ui.add_space(20.0);
            ui.separator();
            ui.label("Configuration file");

            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.file_path).desired_width(360.0));

                if ui.button("Open .nss…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Nilesoft Shell files", &["nss"])
                        .add_filter("All files", &["*"])
                        .pick_file()
                    {
                        self.file_path = path.display().to_string();
                        self.load();
                    }
                }

                if ui.button("Reload").clicked() {
                    self.load();
                }
                if ui.button("Save").clicked() {
                    self.save();
                }
            });

            ui.colored_label(
                egui::Color32::GRAY,
                "Use Open .nss to choose any existing file. Saving overwrites the selected file.",
            );
            ui.colored_label(egui::Color32::GRAY, &self.status);
        });
    }
}

fn show_preview_row(ui: &mut egui::Ui, entry: &Entry, selected: bool) {
    if entry.kind == EntryKind::Separator {
        ui.add_space(2.0);
        ui.separator();
        ui.add_space(2.0);
        return;
    }

    let enabled = entry.kind != EntryKind::Item || entry.enabled;
    ui.add_enabled_ui(enabled, |ui| {
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };

        egui::Frame::none().fill(fill).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(23.0);
            ui.horizontal(|ui| {
                if entry.icon.trim().is_empty() {
                    ui.add_space(18.0);
                } else {
                    ui.add_sized([18.0, 20.0], egui::Label::new(entry.icon.trim()));
                }

                let title = if entry.title.trim().is_empty() {
                    "Untitled entry"
                } else {
                    entry.title.as_str()
                };
                ui.label(title);

                if entry.kind == EntryKind::Menu {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("> ");
                    });
                }
            });
        });
    });
}

fn parse_nss(contents: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut depth = 0usize;
    let mut awaiting_menu_brace = false;

    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('}') {
            depth = depth.saturating_sub(1);
            awaiting_menu_brace = false;
            continue;
        }

        // Some files put the menu's opening brace on the following line.
        if line.starts_with('{') {
            if awaiting_menu_brace {
                depth += 1;
                awaiting_menu_brace = false;
            }
            continue;
        }

        if line.starts_with("separator") {
            entries.push(Entry {
                kind: EntryKind::Separator,
                title: String::new(),
                command: String::new(),
                icon: String::new(),
                enabled: true,
                depth,
            });
        } else if line.starts_with("item(") {
            entries.push(Entry {
                kind: EntryKind::Item,
                title: attr(line, "title"),
                command: attr(line, "command"),
                icon: attr(line, "image"),
                enabled: attr(line, "enabled") != "0",
                depth,
            });
        } else if line.starts_with("menu(") {
            entries.push(Entry {
                kind: EntryKind::Menu,
                title: attr(line, "title"),
                command: String::new(),
                icon: attr(line, "image"),
                enabled: true,
                depth,
            });

            if line.contains('{') {
                depth += 1;
                awaiting_menu_brace = false;
            } else {
                awaiting_menu_brace = true;
            }
        }
    }

    entries
}

fn attr(line: &str, name: &str) -> String {
    // Accept name=value and name = value, with single, double, backtick,
    // or unquoted values.
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        while index < bytes.len()
            && (bytes[index].is_ascii_whitespace() || bytes[index] == b'(' || bytes[index] == b',')
        {
            index += 1;
        }

        let key_start = index;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }

        if key_start == index {
            index += 1;
            continue;
        }

        let key = &line[key_start..index];
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        if key != name || index >= bytes.len() || bytes[index] != b'=' {
            continue;
        }

        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        if index >= bytes.len() {
            return String::new();
        }

        let quote = match bytes[index] {
            b'\'' | b'"' | b'`' => Some(bytes[index]),
            _ => None,
        };

        if quote.is_some() {
            index += 1;
        }

        let mut value = String::new();
        let mut escaped = false;

        while index < bytes.len() {
            let ch = bytes[index] as char;

            if escaped {
                value.push(ch);
                escaped = false;
                index += 1;
                continue;
            }

            if ch == '\\' && quote.is_some() {
                escaped = true;
                index += 1;
                continue;
            }

            if quote == Some(bytes[index])
                || (quote.is_none() && (ch.is_ascii_whitespace() || ch == ')' || ch == ','))
            {
                break;
            }

            value.push(ch);
            index += 1;
        }

        return value;
    }

    String::new()
}

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Shell Context Menu Editor",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1180.0, 720.0])
                .with_min_inner_size([980.0, 560.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
