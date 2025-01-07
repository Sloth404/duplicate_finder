// scanner.rs
use std::sync::Arc;
use tokio::fs;
use tokio::task;
use std::path::Path;
use std::time::Instant;

pub async fn scan_directory(dir: Arc<String>) -> Vec<String> {
    let start_time = Instant::now();
    let mut paths = Vec::new();

    let mut entries = match fs::read_dir(&*dir).await {
        Ok(entries) => entries,
        Err(_) => return paths,
    };

    let mut tasks = vec![];

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            let subdir_path = Arc::new(path.to_str().unwrap().to_string());
            tasks.push(task::spawn(scan_directory(subdir_path)));
        } else if is_image_file(&path) {
            paths.push(path.to_str().unwrap().to_string());
        }
    }

    for task in tasks {
        if let Ok(sub_paths) = task.await {
            paths.extend(sub_paths);
        }
    }

    println!("Scanned directory: {} in {:?}", dir, start_time.elapsed());
    paths
}

fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        matches!(
            ext.to_str().unwrap().to_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "bmp" | "gif" | "tiff"
        )
    } else {
        false
    }
}
