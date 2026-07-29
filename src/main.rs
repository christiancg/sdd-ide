use std::sync::mpsc::{Receiver, Sender};
use eframe::egui::include_image;
use egui_dock::{NodeIndex, SurfaceIndex, TabIndex, TabPath};
use crate::services::services::{AppFile, FileServices};
use crate::ui::name_modal::{NameModal, NameModalResult};
use crate::ui::tab::{EditorTab, MyTabViewer};

mod services;
mod ui;

static EMPTY_STRING: String = String::new();

enum AsyncEventRequest {
    GetFilesAndFolders,
    GetTab(AppFile),
    CreateNewFolder(String),
    CreateNewFile(String, String),
}

enum AsyncEventResponse {
    GetFilesAndFolders(Vec<AppFile>),
    GetTab(EditorTab),
    CreateNewFolder,
    CreateNewFile,
}

struct MainApp {
    tree: egui_dock::DockState<EditorTab>,
    files_and_folders: Vec<AppFile>,
    tx_tokio: Sender<AsyncEventRequest>,
    rx_ui: Receiver<AsyncEventResponse>,
    new_file_modal: Option<NameModal>,
    new_folder_modal: Option<NameModal>,
}

impl MainApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx_to_tokio, rx_from_eframe) = std::sync::mpsc::channel::<AsyncEventRequest>();
        let (tx_to_eframe, rx_from_tokio) = std::sync::mpsc::channel::<AsyncEventResponse>();
        let own_tx = tx_to_tokio.clone();
        let egui_ctx = cc.egui_ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                while let Ok(send_event) = rx_from_eframe.recv() {
                    let tx = tx_to_eframe.clone();
                    let own_tx = own_tx.clone();

                    let ctx = egui_ctx.clone();

                    tokio::spawn(async move {
                        match send_event {
                            AsyncEventRequest::GetFilesAndFolders => {
                                let files = FileServices::get_files_and_folders().await;
                                let _ = tx.send(AsyncEventResponse::GetFilesAndFolders(files));
                            },
                            AsyncEventRequest::GetTab(app_file) => {
                                let edit = EditorTab::new(app_file.clone()).await;
                                let _ = tx.send(AsyncEventResponse::GetTab(edit));
                            },
                            AsyncEventRequest::CreateNewFile(path, name) => {
                                let result = FileServices::create_new_file(path.clone(), name.clone()).await;
                                if result.is_ok() {
                                    let _ = own_tx.send(AsyncEventRequest::GetFilesAndFolders);
                                }
                            },
                            AsyncEventRequest::CreateNewFolder(path) => {
                                let result = FileServices::create_new_folder(path.clone()).await;
                                if result.is_ok() {
                                    let _ = own_tx.send(AsyncEventRequest::GetFilesAndFolders);
                                }
                            }
                        }
                        ctx.request_repaint();
                    });
                }
            });
        });
        let _ = tx_to_tokio.send(AsyncEventRequest::GetFilesAndFolders);
        Self {
            tree: egui_dock::DockState::new(vec![]),
            files_and_folders: vec![],
            tx_tokio: tx_to_tokio,
            rx_ui: rx_from_tokio,
            new_file_modal: None,
            new_folder_modal: None,
        }
    }
}

#[tokio::main]
async fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simple Rust Code Editor",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MainApp::new(cc)))
        }),
    )
}

fn focus_tab(tree: &mut egui_dock::DockState<EditorTab>, file: &AppFile) {
    let mut found_coords: Option<(usize, usize)> = None;
    for (node_index, node) in tree.main_surface().iter().enumerate() {
        if let Some(tabs) = node.tabs() {
            if let Some(tab_index) = tabs.iter().position(|t| t.file.clone().file_name() == file.clone().file_name()) {
                found_coords = Some((node_index, tab_index));
                break;
            }
        }
    }
    if let Some((node_index, tab_index)) = found_coords {
        let surface = SurfaceIndex::main();
        let node_id = NodeIndex(node_index);
        let tab_id = TabIndex(tab_index);
        let _ = tree.set_active_tab(TabPath::new(surface, node_id, tab_id));
    }
}

impl eframe::App for MainApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let _ = eframe::egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(200.0).show(ui, |ui| {
            ui.heading("Files and folders");
            ui.horizontal(|ui| {
                let image_refresh = include_image!("../assets/icons/refresh.svg");
                if ui.button(image_refresh).clicked() {
                    let _ = self.tx_tokio.send(AsyncEventRequest::GetFilesAndFolders);
                }
                let image_new_file = include_image!("../assets/icons/new-file.svg");
                if ui.button(image_new_file).clicked() {
                    self.new_file_modal = Some(NameModal::new("Create new file".to_string()));
                }
                let image_new_folder = include_image!("../assets/icons/new-folder.svg");
                if ui.button(image_new_folder).clicked() {
                    self.new_folder_modal = Some(NameModal::new("Create new folder".to_string()));
                }
            });
            for file in &self.files_and_folders {
                let is_dir = file.is_dir;
                let filename = file.clone().file_name();

                if ui.button(&filename).clicked() && !is_dir {
                    let mut already_opened_tab = false;

                    for node in self.tree.main_surface().iter() {
                        if let Some(tabs_vec) = node.tabs() {
                            if tabs_vec.iter().any(|t| t.file.clone().file_name() == filename) {
                                already_opened_tab = true;
                                break;
                            }
                        }
                    }

                    if already_opened_tab {
                        let file = file;
                        focus_tab(&mut self.tree, file);
                    } else {
                        let _ = self.tx_tokio.send(AsyncEventRequest::GetTab(file.clone()));
                    }
                }
            }
        });
        if let Some(modal) = self.new_file_modal.as_mut() {
            match modal.show_modal(&ctx) {
                NameModalResult::Accepted(name) => {
                    let _ = self.tx_tokio.send(AsyncEventRequest::CreateNewFile(".".to_string(), name));
                    self.new_file_modal = None;
                },
                NameModalResult::Cancelled => {
                    self.new_file_modal = None;
                },
                NameModalResult::Open => {},
            }
        }
        if let Some(modal) = self.new_folder_modal.as_mut() {
            match modal.show_modal(&ctx) {
                NameModalResult::Accepted(name) => {
                    let _ = self.tx_tokio.send(AsyncEventRequest::CreateNewFolder(name.clone()));
                    self.new_folder_modal = None;
                },
                NameModalResult::Cancelled => {
                    self.new_folder_modal = None;
                },
                NameModalResult::Open => {},
            }
        }
        if let Ok(event) = self.rx_ui.try_recv() {
            match event {
                AsyncEventResponse::GetFilesAndFolders(files_and_folders) => {
                    self.files_and_folders = files_and_folders;
                },
                AsyncEventResponse::GetTab(tab) => {
                    let file = &tab.file.clone();
                    self.tree.push_to_focused_leaf(tab);
                    let _ = focus_tab(&mut self.tree, file);
                },
                AsyncEventResponse::CreateNewFolder | AsyncEventResponse::CreateNewFile => {
                    // nothing to do, already emitted an event to reload files and folders
                },
            }
            ctx.request_repaint();
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let mut viewer = MyTabViewer;
            egui_dock::DockArea::new(&mut self.tree)
                .show_inside(ui, &mut viewer);
        });
    }
}