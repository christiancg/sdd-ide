use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, SystemTime};
use egui_dock::TabViewer;
use crate::services::services::{AppFile};

enum AsyncEventRequest {
    SaveFile(AppFile, String),
}

enum AsyncEventResponse {
    SaveFile(Message),
}

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

pub struct EditorTab {
    pub file:  AppFile,
    edited_content: String,
    message: Option<Message>,
    tx_tokio: Sender<AsyncEventRequest>,
    rx_ui: Receiver<AsyncEventResponse>,
}

impl EditorTab {
    pub async fn new(file: AppFile) -> Self {
        let (tx_to_tokio, rx_from_ui) = std::sync::mpsc::channel::<AsyncEventRequest>();
        let (tx_to_ui, rx_from_tokio) = std::sync::mpsc::channel::<AsyncEventResponse>();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                while let Ok(send_event) = rx_from_ui.recv() {
                    let tx = tx_to_ui.clone();
                    tokio::spawn(async move {
                        match send_event {
                            AsyncEventRequest::SaveFile(app_file, edited_content) => {
                                let result = app_file.save(&edited_content).await;
                                let message: Option<Message>;
                                if result.is_ok() {
                                    message = Some(Message::new(String::from("Successfully saved file"), 5));
                                } else {
                                    message = Some(Message::new(String::from("Failed to save file"), 5));
                                }
                                let _ = tx.send(AsyncEventResponse::SaveFile(message.unwrap()));
                            },
                        }
                    });
                }
            });
        });
        let read_file = file.clone().read().await;
        if let Ok(content) = read_file {
            Self {
                file,
                edited_content: content.clone(),
                message: None,
                tx_tokio: tx_to_tokio,
                rx_ui: rx_from_tokio,
            }
        } else {
            Self {
                file,
                edited_content: String::default(),
                message: None,
                tx_tokio: tx_to_tokio,
                rx_ui: rx_from_tokio,
            }
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
        if let Ok(event) = tab.rx_ui.try_recv() {
            match event {
                AsyncEventResponse::SaveFile(message) => {
                    tab.message = Some(message);
                },
            }
        }
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
            let file = tab.file.clone();
            let tx_tokio = tab.tx_tokio.clone();
            let _ = tx_tokio.send(AsyncEventRequest::SaveFile(file, tab.edited_content.clone()));
        }
        if tab.message.is_some() {
            let message = tab.message.as_ref().unwrap();
            ui.label(message.message.clone());
        }
    }
}