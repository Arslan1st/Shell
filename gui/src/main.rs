use eframe::egui;
use std::{fs, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryKind { Item, Menu, Separator }

#[derive(Clone)]
struct Entry { kind: EntryKind, title: String, command: String, icon: String, enabled: bool }

impl Entry {
    fn label(&self) -> String {
        match self.kind { EntryKind::Separator => "Separator".into(), _ if self.title.is_empty() => "Untitled entry".into(), _ => self.title.clone() }
    }
    fn to_nss(&self, indent: &str) -> String {
        match self.kind {
            EntryKind::Separator => format!("{indent}separator\n"),
            EntryKind::Item => format!("{indent}item(title = '{}' command = '{}' image = '{}' enabled = {})\n", esc(&self.title), esc(&self.command), esc(&self.icon), self.enabled as u8),
            EntryKind::Menu => format!("{indent}menu(title = '{}' image = '{}') {{\n{indent}}}\n", esc(&self.title), esc(&self.icon)),
        }
    }
}

fn esc(value: &str) -> String { value.replace('\\', "\\\\").replace('\'', "\\'") }

struct App {
    entries: Vec<Entry>, selected: Option<usize>, file_path: String, status: String,
}

impl Default for App {
    fn default() -> Self { Self { entries: vec![Entry { kind: EntryKind::Item, title: "Open in Terminal".into(), command: "wt.exe -d \"%sel.path\"".into(), icon: "terminal".into(), enabled: true }], selected: Some(0), file_path: "../src/bin/imports/gui.nss".into(), status: "Ready".into() } }
}

impl App {
    fn selected_mut(&mut self) -> Option<&mut Entry> { self.selected.and_then(|i| self.entries.get_mut(i)) }
    fn save(&mut self) {
        let contents = self.entries.iter().map(|e| e.to_nss("")).collect::<String>();
        match fs::write(PathBuf::from(&self.file_path), contents) { Ok(_) => self.status = format!("Saved {} entries", self.entries.len()), Err(e) => self.status = format!("Save failed: {e}") }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| { ui.heading("Shell Context Menu"); ui.separator(); ui.label("Rust editor"); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { if ui.button("Save changes").clicked() { self.save(); } }); });
        });
        egui::SidePanel::left("entries").resizable(true).default_width(250.0).show(ctx, |ui| {
            ui.heading("Menu entries"); ui.add_space(8.0);
            ui.horizontal(|ui| { if ui.button("+ Item").clicked() { self.entries.push(Entry { kind: EntryKind::Item, title: "New item".into(), command: String::new(), icon: String::new(), enabled: true }); self.selected = Some(self.entries.len()-1); }
                if ui.button("+ Menu").clicked() { self.entries.push(Entry { kind: EntryKind::Menu, title: "New menu".into(), command: String::new(), icon: String::new(), enabled: true }); self.selected = Some(self.entries.len()-1); }
                if ui.button("+ Separator").clicked() { self.entries.push(Entry { kind: EntryKind::Separator, title: String::new(), command: String::new(), icon: String::new(), enabled: true }); self.selected = Some(self.entries.len()-1); } });
            ui.separator();
            let mut remove = None;
            for (i, entry) in self.entries.iter().enumerate() { ui.horizontal(|ui| { let selected = self.selected == Some(i); if ui.selectable_label(selected, entry.label()).clicked() { self.selected = Some(i); } if selected && ui.small_button("×").clicked() { remove = Some(i); } }); }
            if let Some(i) = remove { self.entries.remove(i); self.selected = self.entries.len().checked_sub(1); }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0); ui.heading("Entry properties"); ui.add_space(12.0);
            if let Some(entry) = self.selected_mut() {
                ui.horizontal(|ui| { ui.label("Type"); ui.radio_value(&mut entry.kind, EntryKind::Item, "Item"); ui.radio_value(&mut entry.kind, EntryKind::Menu, "Menu"); ui.radio_value(&mut entry.kind, EntryKind::Separator, "Separator"); });
                ui.add_space(14.0);
                if entry.kind != EntryKind::Separator { ui.label("Title"); ui.text_edit_singleline(&mut entry.title); ui.add_space(8.0); ui.label("Icon / glyph"); ui.text_edit_singleline(&mut entry.icon); }
                if entry.kind == EntryKind::Item { ui.add_space(8.0); ui.label("Command"); ui.add(egui::TextEdit::multiline(&mut entry.command).desired_rows(4).hint_text("Example: wt.exe -d %sel.path")); ui.checkbox(&mut entry.enabled, "Enabled"); }
                ui.separator(); ui.label("Preview"); ui.code(entry.to_nss(""));
            } else { ui.colored_label(egui::Color32::GRAY, "Select an entry or add one from the left."); }
            ui.add_space(20.0); ui.separator(); ui.label("Configuration file"); ui.horizontal(|ui| { ui.add(egui::TextEdit::singleline(&mut self.file_path).desired_width(450.0)); if ui.button("Save").clicked() { self.save(); } }); ui.colored_label(egui::Color32::GRAY, &self.status);
        });
    }
}

fn main() -> eframe::Result<()> {
    eframe::run_native("Shell Context Menu Editor", eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 620.0]).with_min_inner_size([720.0, 480.0]), ..Default::default() }, Box::new(|_cc| Ok(Box::new(App::default()))))
}
