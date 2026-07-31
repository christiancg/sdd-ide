use std::sync::mpsc::{Receiver, Sender};
use eframe::egui::include_image;
use egui_dock::{NodeIndex, SurfaceIndex, TabIndex, TabPath};
use egui_ltreeview::{Action, NodeBuilder, TreeView, TreeViewBuilder, TreeViewState};
use crate::services::services::{AppFile, FileServices};
use crate::ui::name_modal::{NameModal, NameModalResult};
use crate::ui::tab::{EditorTab, MyTabViewer};

mod services;
mod ui;


enum AsyncEventRequest {
    GetFilesAndFolders,
    GetTab(AppFile),
    CreateNewFolder(String, String),
    CreateNewFile(String, String),
    Delete(String, bool),
    Move(Vec<String>, String),
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
    new_file_modal: Option<NameModal>,
    new_folder_modal: Option<NameModal>,
    show_hidden_files: bool,
    collapse_all_requested: bool,
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
                                let files = FileServices::get_files_and_folders(".".to_string()).await;
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
                            AsyncEventRequest::CreateNewFolder(path, name) => {
                                let result = FileServices::create_new_folder(path.clone(), name).await;
                                if result.is_ok() {
                                    let _ = own_tx.send(AsyncEventRequest::GetFilesAndFolders);
                                }
                            },
                            AsyncEventRequest::Delete(path, is_dir) => {
                                let result = FileServices::delete(path.clone(), is_dir).await;
                                if result.is_ok() {
                                    let _ = own_tx.send(AsyncEventRequest::GetFilesAndFolders);
                                }
                            },
                            AsyncEventRequest::Move(paths, destination) => {
                                let result = FileServices::move_files(paths, destination).await;
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
            show_hidden_files: false,
            collapse_all_requested: false,
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

fn file_tree(app: &mut MainApp, files: Vec<AppFile>, builder: &mut TreeViewBuilder<String>) {
    for file in &files {
        let is_dir = file.is_dir;
        let filename = file.clone().file_name();
        if !app.show_hidden_files && file.is_hidden {
            continue;
        }
        let mut node: NodeBuilder<String>;
        if is_dir {
            node = NodeBuilder::dir(file.path.clone()).default_open(false);
        } else {
            node = NodeBuilder::leaf(file.path.clone());
        }
        node = node.label(filename.clone()).context_menu(get_context_menu(is_dir, app, file.clone()));
        if is_dir {
            let is_open = builder.node(node);
            if is_open {
                if let Some(ref children) = file.children {
                    file_tree(app, children.clone(), builder);
                }
            }
            builder.close_dir();
        } else {
            builder.node(node);
        }
    }
}

fn get_context_menu<'a>(is_dir: bool, app: &'a mut MainApp, file: AppFile) -> impl FnMut(&mut egui::Ui) + 'a {
    move |ui| {
        if ui.button("Delete").clicked() {
            let _ = app.tx_tokio.send(AsyncEventRequest::Delete(file.path.clone(), is_dir));
            ui.close();
        }
        if is_dir && ui.button("Create new file").clicked() {
            app.new_file_modal = Some(NameModal::new_with_path("Create new file".to_string(), file.path.clone()));
            ui.close();
        }
        if is_dir && ui.button("Create new folder").clicked() {
            app.new_folder_modal = Some(NameModal::new_with_path("Create new folder".to_string(), file.path.clone()));
            ui.close();
        }
    }
}

fn search_app_file(files_and_folders: Vec<AppFile>, name: String) -> Option<AppFile> {
    for file in files_and_folders.clone() {
        if file.clone().path == name {
            return Some(file);
        } else if file.is_dir {
            if let Some(children) = file.children {
                let result = search_app_file(children, name.clone());
                if result.is_some() {
                    return result;
                }
            }
        }
    }
    None
}

fn collect_dir_paths(files: &[AppFile], paths: &mut Vec<String>) {
    for file in files {
        if file.is_dir {
            paths.push(file.path.clone());
            if let Some(ref children) = file.children {
                collect_dir_paths(children, paths);
            }
        }
    }
}

fn show_treeview(app: &mut MainApp, ui: &mut egui::Ui, files: Vec<AppFile>) {
    let id = ui.make_persistent_id("files and folders tree view");
    let mut state = TreeViewState::load(ui, id).unwrap_or_default();
    if app.collapse_all_requested {
        let mut dir_paths = Vec::new();
        collect_dir_paths(&files, &mut dir_paths);
        for path in dir_paths {
            state.set_openness(path, false);
        }
        app.collapse_all_requested = false;
    }
    let (_, actions) = TreeView::new(id).show_state(ui, &mut state, |builder| {
        file_tree(app, files.clone(), builder);
    });
    state.store(ui, id);
    for action in actions.iter() {
        match action {
            Action::Move(move_dir) => {
                let _ = app.tx_tokio.send(AsyncEventRequest::Move(move_dir.source.clone(), move_dir.target.clone()));
            }
            Action::SetSelected(_) => {}
            Action::Drag(_dnd) => {}
            Action::Activate(activate) => {
                for node_id in activate.selected.clone() {
                    let found_file: Option<AppFile> = search_app_file(app.files_and_folders.clone(), node_id.clone());
                    if let Some(found_file) = found_file {
                        if !found_file.is_dir {
                            let mut already_opened_tab = false;
                            for node in app.tree.main_surface().iter() {
                                if let Some(tabs_vec) = node.tabs() {
                                    if tabs_vec.iter().any(|t| t.file.path == node_id) {
                                        already_opened_tab = true;
                                        break;
                                    }
                                }
                            }
                            if already_opened_tab {
                                let file = found_file;
                                focus_tab(&mut app.tree, &file);
                            } else {
                                let _ = app.tx_tokio.send(AsyncEventRequest::GetTab(found_file.clone()));
                            }
                        }
                    }
                }
            }
            Action::DragExternal(..) => {}
            Action::MoveExternal(..) => {}
        }
    }
}

impl eframe::App for MainApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let _ = egui::Panel::left("sidebar")
            .resizable(true)
            .default_size(230.0).show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
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
                        ui.toggle_value(&mut self.show_hidden_files, "Show hidden");
                        let image_collapse = include_image!("../assets/icons/collapse.svg");
                        if ui.button(image_collapse).clicked() {
                            self.collapse_all_requested = true;
                        }
                    });
                    show_treeview(self, ui, self.files_and_folders.clone());
                });
            });
        if let Some(modal) = self.new_file_modal.as_mut() {
            match modal.show_modal(&ctx) {
                NameModalResult::Accepted(path, name) => {
                    let _ = self.tx_tokio.send(AsyncEventRequest::CreateNewFile(path, name));
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
                NameModalResult::Accepted(path, name) => {
                    let _ = self.tx_tokio.send(AsyncEventRequest::CreateNewFolder(path, name));
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