use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use image::{DynamicImage, imageops};
use image::ImageReader;
use tokio::fs;
use tokio::task;
use tokio_stream::StreamExt;
use std::time::Instant;
use futures::future::join_all;
use crate::file_scanner::scan_directory;

#[derive(Debug, Clone)]
pub struct Progress {
    pub progress: f32,
}

impl Default for Progress {
    fn default() -> Self {
        Progress {
            progress: 0.0, // Set the default value for progress
        }
    }
}

pub async fn find_duplicates(directory: &str, progress: Arc<Mutex<Progress>>, output_file: &str) -> Vec<String> {
    let start_time = Instant::now();
    println!("Starting to find duplicates in directory: {}", directory);

    let image_paths = collect_image_paths(&directory.to_string()).await;
    let total = image_paths.len();
    println!("Found {} image(s) to process.", total);

    if total == 0 {
        return Vec::new();
    }

    let pb_increment = 1.0 / total as f32;

    // Process images concurrently
    let start_process_time = Instant::now();
    let hash_map = process_images_concurrently(&image_paths, &progress, pb_increment).await;
    let process_duration = start_process_time.elapsed();
    println!("Image processing took: {:?}", process_duration);

    let mut duplicates = Vec::new();
    for (_hash, paths) in hash_map.iter() {
        if paths.len() > 1 {
            duplicates.push(paths[0].clone());
        }
    }

    println!("Found {} duplicate(s).", duplicates.len());

    let total_duration = start_time.elapsed();
    println!("Total time for finding duplicates: {:?}", total_duration);

    duplicates
}

// Optimized directory scanning (parallelized)
async fn collect_image_paths(dir: &str) -> Vec<String> {
    let start_time = Instant::now();
    let mut paths = Vec::new(); // To store the image paths

    // Use the scan_directory from scanner.rs
    let sub_paths = scanner::scan_directory(Arc::new(dir.to_owned())).await;
    paths.extend(sub_paths); // Add subdirectory results to paths

    let duration = start_time.elapsed();
    println!("Directory scanning took: {:?}", duration);

    paths
}

// Using difference hash (dHash) for faster hashing
async fn compute_image_hash(path: &str) -> Result<String, String> {
    let start_time = Instant::now();
    println!("Computing hash for image: {}", path);

    let path = path.to_string();
    let path_string = path.clone();

    let img = tokio::task::spawn_blocking(move || {
        ImageReader::open(&path)
            .map_err(|_| format!("Error opening file: {}", path))?
            .decode()
            .map_err(|_| format!("Error decoding image: {}", path))
    })
    .await
    .unwrap();

    match img {
        Ok(image) => {
            let hash = compute_dhash(&image);
            let duration = start_time.elapsed();
            println!("Hash computation took: {:?}", duration);

            Ok(hash)
        }
        Err(e) => Err(e),
    }
}

// dHash Algorithm (faster and efficient)
fn compute_dhash(img: &DynamicImage) -> String {
    let start_time = Instant::now();
    let gray_img = img.to_luma8();
    let (width, height) = gray_img.dimensions();

    // Generate dHash (difference hash)
    let mut hash = String::new();
    for y in 0..height {
        for x in 0..(width - 1) {
            let pixel1 = gray_img.get_pixel(x, y).0[0];
            let pixel2 = gray_img.get_pixel(x + 1, y).0[0];
            hash.push(if pixel1 < pixel2 { '1' } else { '0' });
        }
    }

    let duration = start_time.elapsed();
    println!("dHash computation took: {:?}", duration);

    hash
}

// Caching images asynchronously to avoid redundant reads
async fn process_images_concurrently(
    image_paths: &[String],
    progress: &Arc<Mutex<Progress>>,
    pb_increment: f32,
) -> HashMap<String, Vec<String>> {
    let hash_map = Arc::new(Mutex::new(HashMap::new()));
    let mut tasks = Vec::new();

    // Caching and processing images concurrently
    for path in image_paths {
        let hash_map = Arc::clone(&hash_map);
        let progress = Arc::clone(&progress);
        let path = path.clone();

        let task = tokio::spawn(async move {
            if let Ok(hash) = compute_image_hash(&path).await {
                let mut map = hash_map.lock().unwrap();
                map.entry(hash).or_insert_with(Vec::new).push(path);
            }

            // Update progress
            let mut prog = progress.lock().unwrap();
            prog.progress += pb_increment;
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let _ = join_all(tasks).await;

    // Return the hash map containing all the results
    Arc::try_unwrap(hash_map)
        .unwrap_or_else(|_| panic!("Failed to unwrap hash_map Arc"))
        .into_inner()
        .unwrap_or_else(|_| panic!("Failed to access hash_map data"))
}
