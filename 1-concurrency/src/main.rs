use clap::{Parser, ValueEnum};
use regex::bytes::Regex;
use std::path::PathBuf;

use std::collections::BTreeMap;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// The the struct you need to use to print your results.
pub use crate::grep_result::GrepResult;

mod grep_result;

/// Kind selector for the bonus assignment
#[derive(Debug, Default, Clone, Copy, ValueEnum)]
enum Kind {
    #[default]
    SelfMade,
    Rayon,
    Tokio,
}

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// The regex pattern that the user provided
    regex: String,

    /// Which version of the grep to run
    #[arg(default_value = "self-made", short, long)]
    kind: Kind,

    /// The paths in which mygrep should search, if empty, in the current directory
    paths: Vec<String>,
}

fn main() {
    //Parse arguments, using the clap crate
    let args: Args = Args::parse();
    let regex = Regex::new(&args.regex).unwrap();

    // Get the paths that we should search
    let paths = if args.paths.is_empty() {
        //If no paths were provided, we search the current path
        vec![std::env::current_dir().unwrap()]
    } else {
        // Take all paths from the command line arguments, and map the paths to create PathBufs
        args.paths.iter().map(PathBuf::from).collect()
    };

    match args.kind {
        Kind::SelfMade => run_self_made(regex, paths),
        Kind::Rayon => {
            eprintln!("Rayon version not implemented yet");
        }
        Kind::Tokio => {
            eprintln!("Tokio version not implemented yet");
        }
    }
}

/// Recursively walk a path and send all files into the work queue.
fn walk_path(path: PathBuf, tx: &mpsc::Sender<PathBuf>) {
    if let Ok(metadata) = fs::metadata(&path) {
        if metadata.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    walk_path(entry.path(), tx);
                }
            }
        } else if metadata.is_file() {
            // Ignore errors if the receiver is gone
            let _ = tx.send(path);
        }
    }
}

/// Self-made concurrent grep implementation.
fn run_self_made(regex: Regex, roots: Vec<PathBuf>) {
    // Shared regex and search counter
    let regex = Arc::new(regex);
    let counter = Arc::new(AtomicUsize::new(0));

    // Channel for work items (file paths)
    let (path_tx, path_rx) = mpsc::channel::<PathBuf>();
    // Channel for results
    let (res_tx, res_rx) = mpsc::channel::<GrepResult>();

    // Wrap receiver so multiple workers can pull from it safely
    let path_rx = Arc::new(Mutex::new(path_rx));

    // Spawn printer thread first, it will consume GrepResults as they come in
    let printer_handle = thread::spawn(move || {
        let mut next_id: usize = 0;
        let mut buffer: BTreeMap<usize, GrepResult> = BTreeMap::new();

        // Receive results until all senders are dropped
        while let Ok(res) = res_rx.recv() {
            buffer.insert(res.search_ctr, res);

            // Print all consecutive ready results
            loop {
                if let Some(r) = buffer.remove(&next_id) {
                    println!("{}", r);
                    next_id += 1;
                } else {
                    break;
                }
            }
        }

        // Print any remaining buffered results (if any)
        while let Some(r) = buffer.remove(&next_id) {
            println!("{}", r);
            next_id += 1;
        }
    });

    // Determine number of worker threads based on available cores
    let num_workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Spawn worker threads
    let mut worker_handles = Vec::new();
    for _ in 0..num_workers {
        let path_rx = Arc::clone(&path_rx);
        let res_tx = res_tx.clone();
        let regex = Arc::clone(&regex);
        let counter = Arc::clone(&counter);

        let handle = thread::spawn(move || loop {
            // Get next path from the queue
            let path = {
                let rx_lock = path_rx.lock().unwrap();
                match rx_lock.recv() {
                    Ok(p) => p,
                    Err(_) => break, // sender closed -> no more work
                }
            };

            // Read entire file as bytes
            let content = match fs::read(&path) {
                Ok(c) => c,
                Err(_) => continue, // skip unreadable files
            };

            // Collect all match ranges in this file
            let mut ranges = Vec::new();
            for m in regex.find_iter(&content) {
                ranges.push(m.start()..m.end());
            }

            // Only send a result if there was at least one match
            if !ranges.is_empty() {
                let id = counter.fetch_add(1, Ordering::SeqCst);
                let result = GrepResult {
                    path: path.clone(),
                    content,
                    ranges,
                    search_ctr: id,
                };
                // If the printer is gone, just stop sending
                if res_tx.send(result).is_err() {
                    break;
                }
            }
        });

        worker_handles.push(handle);
    }

    // We don't send results from the main thread, so drop its sender
    drop(res_tx);

    // Producer: walk all roots and feed files into the path channel
    for root in roots {
        walk_path(root, &path_tx);
    }
    // Close the work channel so workers exit once done
    drop(path_tx);

    // Wait for all workers to finish
    for handle in worker_handles {
        let _ = handle.join();
    }

    // At this point all worker senders for results are dropped, so printer will finish
    let _ = printer_handle.join();
}

