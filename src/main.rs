use std::sync::mpsc::{Receiver, Sender};
use egui_dock::{NodeIndex, SurfaceIndex, TabIndex};
use crate::services::services::{AppFile, FileServices};
use crate::ui::tab::{EditorTab, MyTabViewer};

mod services;
mod ui;

static EMPTY_STRING: String = String::new();

enum AsyncEventRequest {
    GetFilesAndFolders,
    GetTab(AppFile),
}

enum AsyncEventResponse {
    GetFilesAndFolders(Vec<AppFile>),
    GetTab(EditorTab),
}

struct MainApp {
    tree: egui_dock::DockState<EditorTab>,
    files_and_folders: Vec<AppFile>,
    tx_tokio: Sender<AsyncEventRequest>,
    rx_ui: Receiver<AsyncEventResponse>,
}

impl MainApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx_to_tokio, rx_from_eframe) = std::sync::mpsc::channel::<AsyncEventRequest>();
        let (tx_to_eframe, rx_from_tokio) = std::sync::mpsc::channel::<AsyncEventResponse>();

        let egui_ctx = cc.egui_ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                while let Ok(send_event) = rx_from_eframe.recv() {
                    let tx = tx_to_eframe.clone();
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
        }
    }
}

#[tokio::main]
async fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simple Rust Code Editor",
        options,
        Box::new(|cc| Box::new(MainApp::new(cc))),
    )
}

fn focus_tab(tree: &mut egui_dock::DockState<EditorTab>, file: &AppFile) {
    let mut found_coords: Option<(usize, usize)> = None;
    for (node_index, node) in tree.iter_main_surface_nodes().enumerate() {
        if let egui_dock::Node::Leaf { tabs, .. } = node {
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
        tree.set_active_tab((surface, node_id, tab_id));
    }
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let _ = eframe::egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(200.0).show(ctx, |ui| {
            ui.heading("Files and folders");
            for file in &self.files_and_folders {
                let is_dir = file.is_dir;
                let filename = file.clone().file_name();

                if ui.button(&filename).clicked() && !is_dir {
                    let mut already_opened_tab = false;

                    for node in self.tree.iter_main_surface_nodes() {
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
        if let Ok(evento) = self.rx_ui.try_recv() {
            match evento {
                AsyncEventResponse::GetFilesAndFolders(files_and_folders) => {
                    self.files_and_folders = files_and_folders;
                },
                AsyncEventResponse::GetTab(tab) => {
                    let file = &tab.file.clone();
                    self.tree.push_to_focused_leaf(tab);
                    let _ = focus_tab(&mut self.tree, file);
                },
            }
            ctx.request_repaint();
        }
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let mut viewer = MyTabViewer;
            egui_dock::DockArea::new(&mut self.tree)
                .show_inside(ui, &mut viewer);
        });
    }
}