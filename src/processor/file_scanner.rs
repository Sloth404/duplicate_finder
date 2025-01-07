use futures::{stream, Stream, StreamExt}; // 0.3.1
use std::{io, path::{PathBuf, Path}};
use tokio::fs::{self, DirEntry}; // 0.2.4
use tokio::sync::Semaphore;
use std::sync::Arc;

const MAX_CONCURRENT_TASKS: usize = 10; // Set a limit on concurrent tasks

// This function handles directory traversal.
pub fn visit(path: impl Into<PathBuf>) -> impl Stream<Item = io::Result<DirEntry>> + Send + 'static {
    async fn one_level(path: PathBuf, to_visit: &mut Vec<PathBuf>, semaphore: Arc<Semaphore>) -> io::Result<Vec<DirEntry>> {
        let mut dir = fs::read_dir(path).await?;
        let mut files = Vec::new();

        while let Some(child) = dir.next_entry().await? {
            if child.metadata().await?.is_dir() {
                // Only push directories if there's room in the semaphore
                let _permit = semaphore.acquire().await.unwrap();
                to_visit.push(child.path());
            } else {
                files.push(child)
            }
        }

        Ok(files)
    }

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS)); // Semaphore for limiting concurrency

    stream::unfold(vec![path.into()], move |mut to_visit| {
        let semaphore = semaphore.clone();
        async move {
            if let Some(path) = to_visit.pop() {
                let file_stream = match one_level(path, &mut to_visit, semaphore).await {
                    Ok(files) => stream::iter(files).map(Ok).left_stream(),
                    Err(e) => stream::once(async { Err(e) }).right_stream(),
                };

                Some((file_stream, to_visit))
            } else {
                None
            }
        }
    })
    .flatten()
}

#[tokio::main]
async fn main() {
    let root_path = std::env::args().nth(1).expect("One argument required");
    let paths = visit(root_path);

    paths
        .for_each(|entry| {
            async {
                match entry {
                    Ok(entry) => println!("visiting {:?}", entry.path()),
                    Err(e) => eprintln!("encountered an error: {}", e),
                }
            }
        })
        .await;
}
