use egui::{Context, Id};

pub struct NameModal {
    title: String,
    text: Option<String>,
    name: String
}

pub enum NameModalResult {
    Open,
    Cancelled,
    Accepted(String),
}

impl NameModal {
    pub(crate) fn new(title: String) -> Self {
        Self {
            title,
            text: None,
            name: String::new()
        }
    }
    pub(crate) fn new_with_body(title: String, text: String) -> Self {
        Self {
            title,
            text: Some(text),
            name: String::new()
        }
    }

    /// Must be called every frame while the modal should stay open.
    /// Returns whether the modal was accepted, cancelled, or is still open.
    pub fn show_modal(&mut self, ctx: &Context) -> NameModalResult {
        let modal = egui::Modal::new(Id::new("my_modal"));
        let response = modal.show(ctx, |ui| {
            ui.heading(self.title.as_str());
            if let Some(text) = &self.text {
                ui.label(text);
            }
            ui.separator();
            ui.text_edit_singleline(&mut self.name);

            let mut result = NameModalResult::Open;
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    result = NameModalResult::Cancelled;
                    ui.close();
                }
                if ui.button("Accept").clicked() {
                    result = NameModalResult::Accepted(self.name.clone());
                    ui.close();
                }
            });
            result
        });

        if response.should_close() {
            match response.inner {
                NameModalResult::Open => NameModalResult::Cancelled,
                result => result,
            }
        } else {
            NameModalResult::Open
        }
    }
}