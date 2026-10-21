// Project: protext-scraper
// File: scanning.rs
// Description: Bounded workers, completion ordering, and progressive saves.
// Author: Jan Alexandr Kopřiva
// License: MIT

use crate::{
    network::Network,
    storage::{save_articles_progressively, write_articles},
};
use serde_json::Value;
use std::{
    collections::HashSet,
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
            || self.workers > 256
            || self.batch_size > 1_000_000
            || usize::try_from(i128::from(self.max) - i128::from(self.min) + 1).is_err()
            || self.batch_size == 0
            || self.save_frequency == 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid scan range or sizes (workers: 1..=256, batch: 1..=1000000; step and save frequency must be positive)",
            ));
        }
        Ok(())
    }
}
fn report_failed(network: &Network) {
    let mut failed = network.failed.lock().unwrap();
    if !failed.is_empty() {
        failed.sort_unstable();
        println!(
            "WARNING: {} IDs could not be fetched after retries (rate limit or network), so the results are incomplete. First: {:?}",
            failed.len(),
            &failed[..failed.len().min(20)]
        );
        failed.clear();
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
fn parallel<I: Iterator<Item = i64> + Send>(
    network: &Network,
    ids: I,
    options: &ScanOptions,
    processed: &Mutex<HashSet<i64>>,
    progress: bool,
) -> io::Result<Vec<Value>> {
    let queue = Mutex::new(ids);
    let (tx, rx) = mpsc::channel();
    thread::scope(|scope| {
        for _ in 0..options.workers {
            let tx = tx.clone();
            let queue = &queue;
            thread::Builder::new().spawn_scoped(scope, move || loop {
                let id = queue.lock().unwrap().next();
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
            })?;
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
        Ok(articles)
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
    let total = (i128::from(max) - i128::from(min) + 1) as usize;
    let batches = total.div_ceil(batch_size);
    println!("Total range: {total} IDs, Processing in {batches} batches of {batch_size}");
    println!("Saving every {} articles", options.save_frequency);
    initialize(options)?;
    let processed = Mutex::new(HashSet::new());
    let mut all = Vec::new();
    let mut last_save = 0;
    for batch in 0..batches {
        let ids = if options.reverse {
            let start = (i128::from(max) - (batch as i128 * batch_size as i128)) as i64;
            ((i128::from(start) - batch_size as i128 + 1).max(min as i128) as i64..=start)
                .rev()
                .step_by(step)
                .collect::<Vec<_>>()
        } else {
            let start = (i128::from(min) + (batch as i128 * batch_size as i128)) as i64;
            (start..=(i128::from(start) + batch_size as i128 - 1).min(max as i128) as i64)
                .step_by(step)
                .collect::<Vec<_>>()
        };
        println!(
            "\nProcessing batch {}/{batches}: IDs {}-{} ({direction})",
            batch + 1,
            ids[0],
            ids[ids.len() - 1]
        );
        let found = parallel(network, ids.into_iter(), options, &processed, false)?;
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
    report_failed(network);
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
        (options.min..=options.max).step_by(options.step),
        options,
        &Mutex::new(HashSet::new()),
        true,
    )?;
    report_failed(network);
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
    report_failed(network);
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
