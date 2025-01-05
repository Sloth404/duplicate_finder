use std::collections::HashMap; // Import HashMap for storing hash and file paths.
use std::fs::{self, File}; // File system operations and file handling.
use std::io::Write; // To write data into files.
use std::path::Path; // For working with file paths.
use rayon::prelude::*; // Rayon for parallel iteration.
use sha2::{Digest, Sha256}; // SHA-256 hash function from the `sha2` crate.
use image::io::Reader as ImageReader; // For reading image files.
use image::DynamicImage; // Image type from the `image` crate.
use std::sync::{Arc, Mutex}; // Arc and Mutex for thread-safe shared data.
use indicatif::{ProgressBar, ProgressStyle}; // Progress bar for tracking progress.

// Main function
fn main() {
    let directory = "example/path"; // Specify the directory to scan for images.
    let output_file = "duplicates.txt"; // Output file to save duplicate results.

    // Collect all image file paths from the specified directory.
    let image_paths: Vec<String> = collect_image_paths(directory);

    // Initialize a progress bar for tracking progress.
    let pb = ProgressBar::new(image_paths.len() as u64); // Set the progress bar size.
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
        .expect("Invalid progress bar template")
        .progress_chars("#>-"));

    // Create a HashMap to store image hashes and associated file paths.
    let hash_map = process_images(&image_paths, &pb);

    // Finish the progress bar with a message.
    pb.finish_with_message("Hashing completed.");

    // Write duplicate images to the output file.
    write_duplicates_to_file(&hash_map, output_file).expect("Error writing to file");
}

// Function to collect all image paths in a directory (recursively).
fn collect_image_paths(dir: &str) -> Vec<String> {
    let mut paths = Vec::new(); // Vector to store file paths.

    // Read the directory entries.
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                println!("Scanning: {}", entry.path().display()); // Print scanned path.
                let path = entry.path();
                if path.is_dir() {
                    // If it's a directory, recursively collect image paths.
                    paths.extend(collect_image_paths(path.to_str().unwrap()));
                } else if is_image_file(&path) {
                    // If it's an image file, add its path to the vector.
                    paths.push(path.to_str().unwrap().to_string());
                }
            }
        }
    }

    paths // Return the collected paths.
}

// Function to check if a file is an image based on its extension.
fn is_image_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        matches!(ext.to_str().unwrap().to_lowercase().as_str(), "jpg" | "jpeg" | "png" | "bmp" | "gif" | "tiff")
    } else {
        false
    }
}

// Function to compute the SHA-256 hash of an image.
fn compute_image_hash(path: &str) -> Result<String, String> {
    println!("Processing file: {}", path); // Print the file being processed.
    let img = ImageReader::open(path) // Open the image file.
        .map_err(|_| format!("Error opening file: {}", path))?
        .decode() // Decode the image.
        .map_err(|_| format!("Error decoding image: {}", path))?;

    let hash = Sha256::digest(image_to_bytes(&img)); // Compute the hash of the image data.
    Ok(format!("{:x}", hash)) // Return the hash as a hexadecimal string.
}

// Function to convert an image into a byte array for hashing.
fn image_to_bytes(img: &DynamicImage) -> Vec<u8> {
    img.to_rgba8().as_raw().to_vec() // Convert to RGBA and extract raw bytes.
}

// Function to process images and store their hashes in a HashMap.
fn process_images(image_paths: &[String], pb: &ProgressBar) -> HashMap<String, Vec<String>> {
    let hash_map = Arc::new(Mutex::new(HashMap::new())); // Thread-safe HashMap.

    // Process images in parallel.
    image_paths.par_iter().for_each(|path| {
        if let Ok(hash) = compute_image_hash(path) {
            let mut map = hash_map.lock().unwrap(); // Lock the HashMap for thread-safe access.
            map.entry(hash).or_insert_with(Vec::new).push(path.clone()); // Add the path under its hash.
        }
        pb.inc(1); // Increment the progress bar.
    });

    // Return the HashMap after unlocking it.
    Arc::try_unwrap(hash_map).unwrap().into_inner().unwrap()
}

// Function to write duplicate image paths to a file.
fn write_duplicates_to_file(hash_map: &HashMap<String, Vec<String>>, output_file: &str) -> std::io::Result<()> {
    let mut file = File::create(output_file)?; // Create the output file.

    // Iterate through the HashMap.
    for (_hash, paths) in hash_map.iter() {
        if paths.len() > 1 { // Only consider hashes with more than one file path.
            writeln!(file, "Duplicate images:")?; // Write a header for duplicates.
            for path in paths {
                writeln!(file, "{}", path)?; // Write each duplicate file path.
            }
            writeln!(file, "")?; // Add a blank line between groups.
        }
    }

    Ok(()) // Indicate successful write.
}
