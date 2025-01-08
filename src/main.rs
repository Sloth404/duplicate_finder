mod processor;
mod progress;

use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use crate::progress::Progress;

#[derive(Default, Clone)]
struct MyApp {
    directory: String,
    status: String,
    progress: Arc<Mutex<Progress>>,  // Shared progress state
    processing: bool,
    duplicates: Vec<String>,
    notify: Arc<Notify>,  // Used to notify the UI about progress changes
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Directory:");
                ui.text_edit_singleline(&mut self.directory);

                if ui.button("Start").clicked() && !self.processing {
                    if self.directory.is_empty() {
                        self.status = "Please enter a directory.".to_string();
                    } else {
                        self.start_processing();
                    }
                }
            });

            if self.processing {
                let progress_value = self.progress.lock().unwrap().progress;
                ui.add(egui::ProgressBar::new(progress_value.clamp(0.0, 1.0)).show_percentage());
            }

            ui.label(&self.status);

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("duplicates_grid").show(ui, |ui| {
                    for (i, path) in self.duplicates.iter().enumerate() {
                        if i % 4 == 0 && i > 0 {
                            ui.end_row();
                        }
                        ui.label(path);
                    }
                });
            });
        });

        if self.processing {
            ctx.request_repaint();
        }
    }
}

impl MyApp {
    fn start_processing(&mut self) {
        let directory = self.directory.clone();
        let progress = Arc::clone(&self.progress);
        let notify = Arc::clone(&self.notify);

        self.processing = true;
        self.status = "Processing images...".to_string();

        let app_state = Arc::new(Mutex::new(self.clone()));

        tokio::spawn(async move {
            let output_file = "duplicates.txt";
            let updated_duplicates = processor::find_duplicates(&directory, progress, output_file).await;

            let mut app_lock = app_state.lock().unwrap();
            app_lock.duplicates = updated_duplicates;
            app_lock.status = "Processing completed.".to_string();
            app_lock.processing = false;

            notify.notify_one();
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native("Duplicate Finder", options, Box::new(|_cc| Ok(Box::new(MyApp::default()))))
}
