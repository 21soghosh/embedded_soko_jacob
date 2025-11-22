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

    let args: Args = Args::parse();
    let regex = Regex::new(&args.regex).unwrap();

    let paths = if args.paths.is_empty() {
        vec![std::env::current_dir().unwrap()]
    } else {
        args.paths.iter().map(PathBuf::from).collect()
    };

    match args.kind {
        Kind::SelfMade => run_self_made(regex, paths),
        Kind::Rayon => run_rayon(regex, paths),
        Kind::Tokio => run_tokio(regex, paths)
    }
}

fn walk_path(path: PathBuf, tx: &mpsc::Sender<PathBuf>) {
    if let Ok(metadata) = fs::metadata(&path) {
        if metadata.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.flatten() {
                    walk_path(entry.path(), tx);
                }
            }
        } else if metadata.is_file() {
            let _ = tx.send(path);
        }
    }
}

fn collect_files(path: &PathBuf, out: &mut Vec<PathBuf>) {
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    collect_files(&entry.path(), out);
                }
            }
        } else if metadata.is_file() {
            out.push(path.clone());
        }
    }
}

fn run_self_made(regex: Regex, roots: Vec<PathBuf>) {
    let regex = Arc::new(regex);
    let counter = Arc::new(AtomicUsize::new(0));

    let (path_tx, path_rx) = mpsc::channel::<PathBuf>();
    let (res_tx, res_rx) = mpsc::channel::<GrepResult>();
    let path_rx = Arc::new(Mutex::new(path_rx));
    let printer_handle = thread::spawn(move || {
        let mut next_id: usize = 0;
        let mut buffer: BTreeMap<usize, GrepResult> = BTreeMap::new();

        while let Ok(res) = res_rx.recv() {
            buffer.insert(res.search_ctr, res);

            loop {
                if let Some(r) = buffer.remove(&next_id) {
                    println!("{}", r);
                    next_id += 1;
                } else {
                    break;
                }
            }
        }
        while let Some(r) = buffer.remove(&next_id) {
            println!("{}", r);
            next_id += 1;
        }
    });

    let num_workers = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let mut worker_handles = Vec::new();
    for _ in 0..num_workers {
        let path_rx = Arc::clone(&path_rx);
        let res_tx = res_tx.clone();
        let regex = Arc::clone(&regex);
        let counter = Arc::clone(&counter);

        let handle = thread::spawn(move || loop {
            let path = {
                let rx_lock = match path_rx.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(), 
                };
                match rx_lock.recv() {
                    Ok(p) => p,
                    Err(_) => break, 
                }
            };

            let content = match fs::read(&path) {
                Ok(c) => c,
                Err(_) => continue, 
            };
            let mut ranges = Vec::new();
            for m in regex.find_iter(&content) {
                ranges.push(m.start()..m.end());
            }
            if !ranges.is_empty() {
                let id = counter.fetch_add(1, Ordering::SeqCst);
                let result = GrepResult {
                    path: path.clone(),
                    content,
                    ranges,
                    search_ctr: id,
                };
                if res_tx.send(result).is_err() {
                    break;
                }
            }
        });

        worker_handles.push(handle);
    }
    drop(res_tx);

    for root in roots {
        walk_path(root, &path_tx);
    }
    drop(path_tx);
    for handle in worker_handles {
        let _ = handle.join();
    }
    let _ = printer_handle.join();
}

fn run_rayon(regex: Regex, roots: Vec<PathBuf>) {
    use rayon::prelude::*;

    let regex = Arc::new(regex);
    let counter = Arc::new(AtomicUsize::new(0));

    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }

    let (res_tx, res_rx) = mpsc::channel::<GrepResult>();

    let printer_handle = thread::spawn(move || {
        let mut next_id: usize = 0;
        let mut buffer: BTreeMap<usize, GrepResult> = BTreeMap::new();

        while let Ok(res) = res_rx.recv() {
            buffer.insert(res.search_ctr, res);

            while let Some(r) = buffer.remove(&next_id) {
                println!("{}", r);
                next_id += 1;
            }
        }

        while let Some(r) = buffer.remove(&next_id) {
            println!("{}", r);
            next_id += 1;
        }
    });

    files.par_iter().for_each(|path| {
        let content = match fs::read(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut ranges = Vec::new();
        for m in regex.find_iter(&content) {
            ranges.push(m.start()..m.end());
        }

        if ranges.is_empty() {
            return;
        }

        let id = counter.fetch_add(1, Ordering::SeqCst);
        let result = GrepResult {
            path: path.clone(),
            content,
            ranges,
            search_ctr: id,
        };
        let _ = res_tx.send(result);
    });
    drop(res_tx);
    let _ = printer_handle.join();
}

fn run_tokio(regex: Regex, roots: Vec<PathBuf>) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to build Tokio runtime: {}", e);
            return;
        }
    };

    runtime.block_on(async_run_tokio(regex, roots));
}

async fn async_run_tokio(regex: Regex, roots: Vec<PathBuf>) {
    use tokio::sync::mpsc;
    use tokio::task;

    let regex = Arc::new(regex);
    let counter = Arc::new(AtomicUsize::new(0));

    let mut files = Vec::new();
    for root in &roots {
        collect_files(root, &mut files);
    }

    let (res_tx, mut res_rx) = mpsc::unbounded_channel::<GrepResult>();

    let printer = task::spawn(async move {
        let mut next_id: usize = 0;
        let mut buffer: BTreeMap<usize, GrepResult> = BTreeMap::new();

        while let Some(res) = res_rx.recv().await {
            buffer.insert(res.search_ctr, res);

            while let Some(r) = buffer.remove(&next_id) {
                println!("{}", r);
                next_id += 1;
            }
        }

        while let Some(r) = buffer.remove(&next_id) {
            println!("{}", r);
            next_id += 1;
        }
    });

    let mut handles = Vec::new();
    for path in files {
        let regex = Arc::clone(&regex);
        let counter = Arc::clone(&counter);
        let res_tx = res_tx.clone();

        let handle = task::spawn(async move {
            let content = match tokio::fs::read(&path).await {
                Ok(c) => c,
                Err(_) => return,
            };

            let mut ranges = Vec::new();
            for m in regex.find_iter(&content) {
                ranges.push(m.start()..m.end());
            }

            if ranges.is_empty() {
                return;
            }

            let id = counter.fetch_add(1, Ordering::SeqCst);
            let result = GrepResult {
                path: path.clone(),
                content,
                ranges,
                search_ctr: id,
            };

            let _ = res_tx.send(result);
        });

        handles.push(handle);
    }

    drop(res_tx);
    for handle in handles {
        let _ = handle.await;
    }
    let _ = printer.await;
}



