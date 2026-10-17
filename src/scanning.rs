// Project: protext-scraper
// File: scanning.rs
// Description: Bounded workers, completion ordering, and progressive saves.
// Author: Jan Alexandr Kopřiva
// Based on protext-scraper by Jan Alexandr Kopřiva
// License: MIT

use crate::{
    network::Network,
    storage::{save_articles_progressively, write_articles},
};
use serde_json::Value;
use std::{
    collections::{HashSet, VecDeque},
    io,
    path::{Path, PathBuf},
    sync::{mpsc, Mutex},
    thread,
};
#[derive(Clone)]
pub struct ScanOptions {
    pub min: i64,
    pub max: i64,
    pub step: usize,
    pub workers: usize,
    pub batch_size: usize,
    pub output: Option<(PathBuf, String)>,
    pub reverse: bool,
    pub save_frequency: usize,
    pub selected: Vec<String>,
}
impl ScanOptions {
    pub fn new(min: i64, max: i64) -> Self {
        Self {
            min,
            max,
            step: 1,
            workers: 10,
            batch_size: 500,
            output: None,
            reverse: true,
            save_frequency: 50,
            selected: Vec::new(),
        }
    }
    pub fn validate(&self) -> io::Result<()> {
        if self.min > self.max
            || self.step == 0
            || self.workers == 0
            || self.batch_size == 0
            || self.save_frequency == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid scan range or nonpositive worker, batch, step, or save frequency",
            ));
        }
        Ok(())
    }
}
fn short(text: &str, length: usize) -> String {
    text.chars().take(length).collect()
}
pub fn process_article_id(
    network: &Network,
    id: i64,
    selected: &[String],
    processed: &Mutex<HashSet<i64>>,
) -> Option<Value> {
    if !processed.lock().unwrap().insert(id) {
        println!("✗ ID {id}: Already processed (duplicate)");
        return None;
    }
    let article = network.fetch_article_by_id(id);
    if let Some(article) = article {
        let cat = article
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("Uncategorized");
        if !selected.is_empty() && !selected.iter().any(|s| s == cat) {
            println!("✗ ID {id}: Category '{cat}' not selected");
            return None;
        }
        let keywords = article
            .get("keywords")
            .and_then(Value::as_str)
            .map(|s| format!(" [Keywords: {}...]", short(s, 30)))
            .unwrap_or_default();
        println!(
            "✓ ID {id}: {}...{keywords}",
            short(article["title"].as_str().unwrap_or(""), 50)
        );
        Some(article)
    } else {
        println!("✗ ID {id}: Not found");
        None
    }
}
fn parallel(
    network: &Network,
    ids: Vec<i64>,
    options: &ScanOptions,
    processed: &Mutex<HashSet<i64>>,
    progress: bool,
) -> Vec<Value> {
    let queue = Mutex::new(VecDeque::from(ids));
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..options.workers {
            let tx = tx.clone();
            let queue = &queue;
            scope.spawn(move || loop {
                let id = queue.lock().unwrap().pop_front();
                let Some(id) = id else {
                    break;
                };
                if tx
                    .send(process_article_id(
                        network,
                        id,
                        &options.selected,
                        processed,
                    ))
                    .is_err()
                {
                    break;
                }
            });
        }
        drop(tx);
        let mut articles = Vec::new();
        for article in rx {
            if let Some(a) = article {
                articles.push(a);
            }
            if progress && articles.len() % 100 == 0 {
                println!("Progress: {} articles found so far...", articles.len());
            }
        }
        articles
    })
}
fn save(options: &ScanOptions, articles: &[Value]) -> io::Result<()> {
    if let Some((dir, name)) = &options.output {
        save_articles_progressively(articles, dir, name)?;
    }
    Ok(())
}
fn initialize(options: &ScanOptions) -> io::Result<()> {
    if let Some((dir, name)) = &options.output {
        std::fs::create_dir_all(dir)?;
        // Preserve earlier results rather than truncating them before the first request.
        if !dir.join(name).exists() {
            write_articles(&dir.join(name), &[])?;
        }
    }
    Ok(())
}
pub fn scan_id_range_parallel_batch(
    network: &Network,
    options: &ScanOptions,
) -> io::Result<Vec<Value>> {
    options.validate()?;
    let (min, max, step, workers, batch_size) = (
        options.min,
        options.max,
        options.step,
        options.workers,
        options.batch_size,
    );
    let direction = if options.reverse {
        "NEWEST → OLDEST"
    } else {
        "OLDEST → NEWEST"
    };
    println!("\nBatch parallel scanning ID range: {min} - {max} (step: {step}, workers: {workers}, batch: {batch_size})");
    println!("Direction: {direction}");
    let total = (max - min + 1) as usize;
    let batches = total.div_ceil(batch_size);
    println!("Total range: {total} IDs, Processing in {batches} batches of {batch_size}");
    println!("Saving every {} articles", options.save_frequency);
    initialize(options)?;
    let processed = Mutex::new(HashSet::new());
    let mut all = Vec::new();
    let mut last_save = 0;
    for batch in 0..batches {
        let ids = if options.reverse {
            let start = max - (batch * batch_size) as i64;
            ((start - batch_size as i64 + 1).max(min)..=start)
                .rev()
                .step_by(step)
                .collect::<Vec<_>>()
        } else {
            let start = min + (batch * batch_size) as i64;
            (start..=(start + batch_size as i64 - 1).min(max))
                .step_by(step)
                .collect::<Vec<_>>()
        };
        println!(
            "\nProcessing batch {}/{batches}: IDs {}-{} ({direction})",
            batch + 1,
            ids[0],
            ids[ids.len() - 1]
        );
        let found = parallel(network, ids, options, &processed, false);
        let n = found.len();
        all.extend(found);
        println!(
            "Batch {} complete: Found {n} articles (Total: {})",
            batch + 1,
            all.len()
        );
        if all.len() >= options.save_frequency && all.len() - last_save >= options.save_frequency {
            println!("Saving {} articles to disk...", all.len());
            save(options, &all)?;
            last_save = all.len();
        }
        if batch + 1 < batches {
            let delay = network.transport.random(2.0, 5.0);
            println!("Waiting {delay:.1} seconds before next batch...");
            if batch > 0 && batch % 15 == 0 {
                println!("Renewing Tor circuit for fresh IP...");
                network.transport.renew();
            }
            network.transport.sleep(delay);
        }
    }
    if !all.is_empty() {
        println!("Final save: {} articles", all.len());
        save(options, &all)?;
    }
    println!(
        "\nBatch parallel scan complete: Found {} articles in range {min}-{max}",
        all.len()
    );
    Ok(all)
}
pub fn scan_id_range_parallel(network: &Network, options: &ScanOptions) -> io::Result<Vec<Value>> {
    options.validate()?;
    println!(
        "\nParallel scanning ID range: {} - {} (step: {}, workers: {})",
        options.min, options.max, options.step, options.workers
    );
    initialize(options)?;
    let found = parallel(
        network,
        (options.min..=options.max).step_by(options.step).collect(),
        options,
        &Mutex::new(HashSet::new()),
        true,
    );
    println!(
        "\nParallel scan complete: Found {} articles in range {}-{}",
        found.len(),
        options.min,
        options.max
    );
    Ok(found)
}
pub fn scan_id_range(network: &Network, options: &ScanOptions) -> io::Result<Vec<Value>> {
    options.validate()?;
    println!(
        "\nScanning ID range: {} - {} (step: {})",
        options.min, options.max, options.step
    );
    let mut found = Vec::new();
    for id in (options.min..=options.max).step_by(options.step) {
        print!("Checking ID {id}... ");
        if let Some(a) = network.fetch_article_by_id(id) {
            println!(
                "✓ Found: {}...",
                short(a["title"].as_str().unwrap_or(""), 50)
            );
            found.push(a);
            if options.output.is_some() && found.len() % 5 == 0 {
                save(options, &found)?;
                println!("Saved {} articles to disk", found.len());
            }
        } else {
            println!("✗ Not found");
        }
        network.transport.sleep(network.transport.random(0.2, 0.8));
        if id % 20 == 0 {
            println!("Progress: {id}/{} (found: {})", options.max, found.len());
        }
    }
    if !found.is_empty() {
        save(options, &found)?;
    }
    println!(
        "\nScan complete: Found {} articles in range {}-{}",
        found.len(),
        options.min,
        options.max
    );
    Ok(found)
}
pub fn output_options(options: &mut ScanOptions, dir: &Path, filename: &str) {
    options.output = Some((dir.into(), filename.into()));
}
