mod processor;

use eframe::egui;
use std::sync::{Arc, Mutex};
use tokio::task;
use crate::processor::Progress;


#[derive(Default, Clone)] // Derive Clone for MyApp
struct MyApp {
    directory: String,
    status: String,
    progress: Arc<Mutex<Progress>>,  // Change this line
    processing: bool,
    duplicates: Vec<String>,
    repaint_needed: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Directory:");
                ui.text_edit_singleline(&mut self.directory);
    
                if ui.button("Start").clicked() && !self.processing {
                    println!("Start button clicked.");
                    if self.directory.is_empty() {
                        self.status = "Please enter a directory.".to_string();
                        println!("Directory is empty, please enter a valid directory.");
                    } else {
                        self.start_processing(); // Start processing
                    }
                }
            });
    
            if self.processing {
                // Clone the progress value instead of moving it
                let progress_value = self.progress.lock().expect("Failed to lock progress mutex").clone();
                ui.add(egui::ProgressBar::new(progress_value.progress).show_percentage());
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
    
        // Check if we need to repaint after async task finishes
        if self.repaint_needed {
            println!("Repainting the UI...");
            ctx.request_repaint();
            self.repaint_needed = false; // Reset the flag
        }
    }    
}

impl MyApp {
    fn start_processing(&mut self) {
        let directory = self.directory.clone();
        let progress = Arc::new(Mutex::new(Progress { progress: 0.0 })); // Create Progress here

        self.processing = true;
        self.status = "Processing images...".to_string();

        let app_state: Arc<Mutex<MyApp>> = Arc::new(Mutex::new(self.clone())); // Add type annotation

        // Start async task for processing
        let app_state_clone = Arc::clone(&app_state);
        let progress_for_find_duplicates = Arc::clone(&progress);

        tokio::spawn(async move {
            println!("Async task started.");

            // Call the find_duplicates function from processor.rs
            let output_file = "duplicates.txt"; // You can specify your desired output file
            let updated_duplicates = processor::find_duplicates(&directory, progress_for_find_duplicates, output_file).await;

            let updated_status = "Processing completed.".to_string();

            println!("Async task completed, updating app state...");

            // Modify app state in a thread-safe manner
            let mut app_lock = app_state_clone.lock().unwrap();
            app_lock.duplicates = updated_duplicates;
            app_lock.status = updated_status;
            app_lock.processing = false;

            // Set flag to trigger repaint in the main thread
            app_lock.repaint_needed = true; // Set the flag to true to indicate a repaint is needed

            println!("App state updated.");
        });
    }
}

#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    println!("Starting the app...");
    eframe::run_native("Duplicate Finder", options, Box::new(|_cc| Ok(Box::new(MyApp::default()))))
}
