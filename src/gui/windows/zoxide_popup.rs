use crate::core::i18n::I18n;
use crate::core::utils::text::fuzzy_match;
use crate::core::zoxide::{self, Entry};
use crate::gui::keymap::{self, Action};
use crate::gui::theme::ThemePalette;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

/// Rows shown at once; the rest are reachable by typing a filter.
const MAX_ROWS: usize = 50;
const NOTICE_DURATION: Duration = Duration::from_secs(3);

/// What a worker thread reports back.
enum Message {
    /// Fresh database contents, or `None` when zoxide is unavailable.
    List(Option<Vec<Entry>>),
    Added(PathBuf, bool),
    Removed(PathBuf, bool),
}

/// Keys read at the start of the frame, before the file list can see them.
#[derive(Default)]
struct KeyIntent {
    close: bool,
    enter: bool,
    add: bool,
    remove: bool,
    step: i32,
}

pub struct ZoxidePopup {
    pub open: bool,
    query: String,
    entries: Vec<Entry>,
    loaded: bool,
    selected: usize,
    scroll_to_selected: bool,
    notice: Option<(String, Instant)>,
    intent: KeyIntent,
    tx: Sender<Message>,
    rx: Receiver<Message>,
}

impl Default for ZoxidePopup {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            open: false,
            query: String::new(),
            entries: Vec::new(),
            loaded: false,
            selected: 0,
            scroll_to_selected: false,
            notice: None,
            intent: KeyIntent::default(),
            tx,
            rx,
        }
    }
}

/// Indices into `entries` matching `query`, best first. Every whitespace
/// separated term must fuzzy-match the path; terms found as plain text in the
/// folder name rank above terms found elsewhere in the path, which rank above
/// scattered letters. Ties keep zoxide's order.
fn filter_entries(entries: &[Entry], query: &str) -> Vec<usize> {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut ranked: Vec<(u8, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let path = entry.path.to_string_lossy();
            if !terms.iter().all(|term| fuzzy_match(&path, term)) {
                return None;
            }
            let path = path.to_lowercase();
            let name = entry
                .path
                .file_name()
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let tier = if terms.iter().all(|term| name.contains(term.as_str())) {
                0
            } else if terms.iter().all(|term| path.contains(term.as_str())) {
                1
            } else {
                2
            };
            Some((tier, index))
        })
        .collect();
    ranked.sort_by_key(|(tier, _)| *tier);
    ranked.into_iter().map(|(_, index)| index).collect()
}

impl ZoxidePopup {
    /// Opens the popup and reloads the list in the background.
    pub fn open(&mut self, ctx: &egui::Context) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
        self.loaded = false;
        self.notice = None;
        self.intent = KeyIntent::default();
        self.spawn_refresh(ctx, None);
    }

    /// Runs an optional change (`true` = add, `false` = remove), then reloads
    /// the list, off the UI thread.
    fn spawn_refresh(&self, ctx: &egui::Context, change: Option<(bool, PathBuf)>) {
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if let Some((is_add, path)) = change {
                let message = if is_add {
                    Message::Added(path.clone(), zoxide::add(&path))
                } else {
                    Message::Removed(path.clone(), zoxide::remove(&path))
                };
                let _ = tx.send(message);
            }
            let _ = tx.send(Message::List(zoxide::query_all()));
            ctx.request_repaint();
        });
    }

    /// Consumes the popup's keys before any other widget sees them, so Ctrl+A
    /// and Enter never reach the file list or the text field.
    pub fn handle_keys(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let cmd = egui::Modifiers::COMMAND;
        let none = egui::Modifiers::NONE;
        ctx.input_mut(|i| {
            self.intent.close |=
                i.consume_key(none, egui::Key::Escape) | keymap::consume(i, Action::ZoxideJump);
            self.intent.add |= i.consume_key(cmd, egui::Key::A);
            self.intent.remove |= i.consume_key(cmd, egui::Key::Delete);
            self.intent.enter |= i.consume_key(none, egui::Key::Enter);
            if i.consume_key(none, egui::Key::ArrowDown) {
                self.intent.step += 1;
            }
            if i.consume_key(none, egui::Key::ArrowUp) {
                self.intent.step -= 1;
            }
        });
    }

    fn drain_messages(&mut self, i18n: &I18n) {
        while let Ok(message) = self.rx.try_recv() {
            match message {
                Message::List(entries) => {
                    self.entries = entries.unwrap_or_default();
                    self.loaded = true;
                }
                Message::Added(path, ok) => {
                    let key = if ok { "zoxide_added" } else { "zoxide_failed" };
                    self.set_notice(format!("{} {}", i18n.tr(key), path.display()));
                }
                Message::Removed(path, ok) => {
                    let key = if ok {
                        "zoxide_removed"
                    } else {
                        "zoxide_failed"
                    };
                    self.set_notice(format!("{} {}", i18n.tr(key), path.display()));
                }
            }
        }
    }

    fn set_notice(&mut self, text: String) {
        self.notice = Some((text, Instant::now()));
    }

    /// Draws the popup. Returns the folder to open when the user picks one.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        i18n: &I18n,
        palette: &ThemePalette,
        current_dir: &Path,
    ) -> Option<PathBuf> {
        if !self.open {
            return None;
        }
        self.drain_messages(i18n);
        let missing = zoxide::is_missing();
        let intent = std::mem::take(&mut self.intent);
        let mut chosen: Option<PathBuf> = None;
        let mut close = intent.close;

        let visible: Vec<usize> = filter_entries(&self.entries, &self.query)
            .into_iter()
            .take(MAX_ROWS)
            .collect();
        if !visible.is_empty() {
            let stepped = (self.selected as i64 + intent.step as i64)
                .clamp(0, visible.len() as i64 - 1) as usize;
            self.scroll_to_selected |= stepped != self.selected;
            self.selected = stepped;
        }
        let highlighted = visible.get(self.selected).map(|&i| self.entries[i].clone());

        if !missing {
            if intent.enter
                && let Some(entry) = highlighted.as_ref().filter(|entry| entry.exists)
            {
                chosen = Some(entry.path.clone());
            }
            if intent.add {
                if zoxide::is_addable_path(current_dir) {
                    self.spawn_refresh(ctx, Some((true, current_dir.to_path_buf())));
                } else {
                    self.set_notice(i18n.tr("zoxide_cannot_add"));
                }
            }
            if intent.remove
                && let Some(entry) = &highlighted
            {
                self.spawn_refresh(ctx, Some((false, entry.path.clone())));
            }
        }

        let modal_bg_clicked = egui::Area::new(egui::Id::new("zoxide_modal_bg"))
            .order(egui::Order::Middle)
            .interactable(true)
            .show(ctx, |ui| {
                let rect = ctx.content_rect();
                ui.painter()
                    .rect_filled(rect, 0.0, palette.modal_background_effect_color);
                ui.interact(
                    rect,
                    ui.id().with("zoxide_modal_bg_click"),
                    egui::Sense::click(),
                )
                .clicked()
            })
            .inner;
        close |= modal_bg_clicked;

        let width = 560.0_f32.min(ctx.content_rect().width() - 32.0).max(200.0);
        egui::Window::new(i18n.tr("zoxide_title"))
            .collapsible(false)
            .resizable(false)
            .fixed_size([width, 0.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::popup(&ctx.style_of(ctx.theme()))
                    .corner_radius(egui::CornerRadius::same(palette.large_radius)),
            )
            .show(ctx, |ui| {
                let muted = ui.visuals().weak_text_color();
                if missing {
                    ui.label(
                        egui::RichText::new(i18n.tr("zoxide_not_found")).color(palette.text_normal),
                    );
                    return;
                }

                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text(i18n.tr("zoxide_filter_hint"))
                        .desired_width(f32::INFINITY),
                );
                // Clicking a row must not drop typing focus.
                edit.request_focus();
                if edit.changed() {
                    self.selected = 0;
                    self.scroll_to_selected = true;
                }
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        if visible.is_empty() {
                            let key = if self.loaded {
                                "zoxide_empty"
                            } else {
                                "zoxide_loading"
                            };
                            ui.label(egui::RichText::new(i18n.tr(key)).color(muted));
                        }
                        for (row, &index) in visible.iter().enumerate() {
                            let entry = &self.entries[index];
                            let color = if entry.exists {
                                palette.text_normal
                            } else {
                                muted
                            };
                            let text = format!("{:>7.1}   {}", entry.score, entry.path.display());
                            let response = ui.add_sized(
                                [ui.available_width(), 22.0],
                                egui::Button::selectable(
                                    row == self.selected,
                                    egui::RichText::new(text).color(color),
                                )
                                .truncate(),
                            );
                            if row == self.selected && self.scroll_to_selected {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() && entry.exists {
                                chosen = Some(entry.path.clone());
                            }
                        }
                        self.scroll_to_selected = false;
                    });

                ui.add_space(6.0);
                if let Some((text, shown)) = &self.notice {
                    if shown.elapsed() < NOTICE_DURATION {
                        ui.label(egui::RichText::new(text).color(palette.primary));
                        ctx.request_repaint_after(NOTICE_DURATION - shown.elapsed());
                    } else {
                        self.notice = None;
                    }
                }
                ui.label(egui::RichText::new(i18n.tr("zoxide_hint")).color(muted));
            });

        if chosen.is_some() || close {
            self.open = false;
        }
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(paths: &[&str]) -> Vec<Entry> {
        paths
            .iter()
            .map(|path| Entry {
                score: 1.0,
                path: PathBuf::from(path),
                exists: true,
            })
            .collect()
    }

    #[test]
    fn empty_query_keeps_every_entry_in_order() {
        let list = entries(&["C:\\a", "C:\\b"]);
        assert_eq!(filter_entries(&list, ""), [0, 1]);
    }

    #[test]
    fn folder_name_hits_rank_first() {
        let list = entries(&["C:\\proj\\docs\\x", "C:\\work\\proj", "C:\\p_r_o_j"]);
        assert_eq!(filter_entries(&list, "proj"), [1, 0, 2]);
    }

    #[test]
    fn every_term_must_match() {
        let list = entries(&["C:\\work\\eden", "C:\\play\\eden", "C:\\work\\other"]);
        assert_eq!(filter_entries(&list, "work eden"), [0]);
        assert!(filter_entries(&list, "zzz").is_empty());
    }
}
