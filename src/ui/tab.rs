use std::time::{Duration, SystemTime};
use egui_dock::TabViewer;
use crate::services::services::AppFile;

#[derive(PartialEq, Clone)]
struct Message {
    message: String,
    show_during_seconds: u16,
    first_shown: SystemTime
}

impl Message {
    fn new(message: String, show_during_seconds: u16) -> Self {
        Self {
            message,
            show_during_seconds,
            first_shown: SystemTime::now()
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct EditorTab {
    pub file:  AppFile,
    original_content: String,
    edited_content: String,
    message: Option<Message>,
}

impl EditorTab {
    pub async fn new(file: AppFile) -> Self {
        let read_file = file.clone().read().await;
        if let Ok(content) = read_file {
            Self {
                file,
                original_content: content.clone(),
                edited_content: content.clone(),
                message: None,
            }
        } else {
            Self {
                file,
                original_content: String::default(),
                edited_content: String::default(),
                message: None,
            }
        }
    }

    pub fn default(file: AppFile) -> Self {
        Self {
            file,
            original_content: String::default(),
            edited_content: String::default(),
            message: None,
        }
    }
}

pub struct MyTabViewer;

impl TabViewer for MyTabViewer {
    type Tab = EditorTab;

    // Set the text displayed on the tab handle
    fn title(&mut self, tab: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        tab.file.clone().file_name().into()
    }

    // Define the inner UI layout for each tab panel
    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, tab: &mut Self::Tab) {
        if tab.message.is_some() {
            let unwrapped = tab.message.as_ref().unwrap();
            let elapsed = unwrapped.first_shown.elapsed();
            if elapsed.unwrap() > Duration::from_secs(unwrapped.show_during_seconds as u64) {
                tab.message = None;
            }
        }

        let selected = &tab.file;
        ui.heading(format!("Editing: {}", &selected.path));
        ui.separator();
        egui_dock::egui::ScrollArea::vertical().show(ui, |ui| {
            let editor = egui_dock::egui::TextEdit::multiline(&mut tab.edited_content)
                .font(egui_dock::egui::TextStyle::Monospace) // Code font
                .code_editor()                    // Enables tab key support
                .desired_width(f32::INFINITY)     // Take up all horizontal space
                .desired_rows(30);
            ui.add(editor);
        });

        if ui.button("Save").clicked() {
            let selected_file = &tab.file;
            let _ = &selected_file.clone().save(&tab.edited_content);
            // if result.is_ok() {
            //     tab.message = Some(Message::new(String::from("Successfully saved file"), 5));
            // } else {
            //     tab.message = Some(Message::new(String::from("Failed to save file"), 5));
            // }
        }
        if tab.message.is_some() {
            let message = tab.message.as_ref().unwrap();
            ui.label(message.message.clone());
        }
    }
}