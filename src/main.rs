use std::error::Error;
use egui_dock::Node;
use crate::services::services::FileServices;
use crate::ui::tab::{EditorTab, MyTabViewer};

mod services;
mod ui;

static EMPTY_STRING: String = String::new();

struct MainApp {
    tree: egui_dock::DockState<EditorTab>,
}

impl Default for MainApp {
    fn default() -> Self {
        let tree = egui_dock::DockState::new(vec![]);
        Self { tree }
    }
}


fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simple Rust Code Editor",
        options,
        Box::new(|_| Box::<MainApp>::default()),
    )
}

fn focus_tab(mut app: &mut MainApp, tab: EditorTab) {
    let mut coordenadas_encontradas: Option<(usize, usize)> = None;
    // Recorremos todos los nodos del árbol de forma mutable con sus índices
    for (node_index, node) in app.tree.iter_main_surface_nodes().enumerate() {
        // Filtramos solo los nodos que contienen pestañas (Hojas / Leaf)
        if let egui_dock::Node::Leaf { tabs, active, .. } = node {
            if let Some(tab_index) = tabs.iter().position(|t| t.file.clone().file_name() == tab.file.clone().file_name()) {
                // Guardamos los índices numéricos puros y salimos del bucle
                coordenadas_encontradas = Some((node_index, tab_index));
                break;
            }
        }
    }
    if let Some((node_index, tab_index)) = coordenadas_encontradas {
        // Opcional: También le damos el foco del sistema a esta hoja
        // 3. Convertimos los índices numéricos a los tipos estrictos de egui_dock
        let surface = egui_dock::SurfaceIndex::main();
        let node_id = egui_dock::NodeIndex(node_index);
        let tab_id = egui_dock::TabIndex(tab_index);
        app.tree.set_active_tab((surface, node_id, tab_id));
    }
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {

        let files_and_folders = FileServices::get_files_and_folders();
        // El SidePanel original vuelve a funcionar perfectamente
        eframe::egui::SidePanel::left("sidebar")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Files and folders");
                for file in files_and_folders {
                    let is_dir = file.is_dir;
                    let filename = file.clone().file_name();

                    if ui.button(&filename).clicked() && !is_dir {
                        // 1. Buscamos si el archivo existe en CUALQUIER pestaña de CUALQUIER nodo
                        let mut already_opened = false;

                        for node in self.tree.iter() {
                            if let Some(tabs_vec) = node.tabs() {
                                // Iteramos sobre todos los archivos abiertos en este recuadro
                                if tabs_vec.iter().any(|t| t.file.clone().file_name() == filename) {
                                    already_opened = true;
                                    break;
                                }
                            }
                        }

                        if !already_opened {
                            // 2. Si realmente NO existe, creamos la pestaña y la empujamos
                            let tab = EditorTab::new(file.clone());
                            self.tree.push_to_focused_leaf(tab.clone());

                            // Opcional: También enfocamos de inmediato la pestaña recién creada
                            focus_tab(self, tab);
                        } else {
                            // 3. Si YA existe en algún lugar, simplemente la traemos al frente
                            let pestaña_temporal = EditorTab::new(file.clone());
                            focus_tab(self, pestaña_temporal);
                        }
                    }
                }
            });
        // El CentralPanel nativo de eframe aloja de forma segura tu egui_dock
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            // 1. Instanciamos el visor de pestañas
            let mut viewer = MyTabViewer;

            // 2. Renderizamos el DockArea usando el 'ui' nativo del bloque
            // Ahora compilará perfectamente a la primera porque el tipo de 'ui'
            // coincide al 100% con lo que espera egui_dock
            egui_dock::DockArea::new(&mut self.tree)
                .show_inside(ui, &mut viewer);
        });
    }
}