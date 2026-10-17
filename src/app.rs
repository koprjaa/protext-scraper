// Project: protext-scraper
// File: app.rs
// Description: Original interactive workflow and category reports.
// Author: Jan Alexandr Kopřiva
// License: MIT

use crate::{
    network::Network,
    scanning::{scan_id_range_parallel_batch, ScanOptions},
    storage::*,
};
use chrono::Local;
use serde_json::{json, Map, Value};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    process::Command,
};

pub fn timestamp() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}
pub fn input(reader: &mut dyn BufRead, prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut s = String::new();
    if reader.read_line(&mut s)? == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "End of input"));
    }
    Ok(s.trim().to_string())
}
pub fn start_tor_service(network: &Network) -> bool {
    if network.check_tor_connection() {
        return true;
    }
    println!("Starting Tor service...");
    let mut started = false;
    for (program, args) in [
        ("brew", vec!["services", "start", "tor"]),
        ("sudo", vec!["systemctl", "start", "tor"]),
    ] {
        if Command::new(program)
            .args(args)
            .output()
            .is_ok_and(|r| r.status.success())
        {
            started = true;
            network.transport.sleep(5.0);
            break;
        }
    }
    if !started {
        println!("Could not start Tor service automatically\nPlease install and start Tor manually:\n  brew install tor && brew services start tor\n  sudo apt install tor && sudo systemctl start tor");
        return false;
    }
    network.check_tor_connection()
}
pub fn analyze_categories_from_json(path: &Path) -> (Map<String, Value>, Vec<(String, u64)>) {
    let articles = load_articles(path);
    if articles.is_empty() {
        println!("No articles to analyze in {}", path.display());
        return (Map::new(), Vec::new());
    }
    let categories = count_categories(&articles);
    let sorted = sorted_categories(&categories);
    println!("\nCATEGORY ANALYSIS\nTotal articles: {}\nNumber of categories: {}\n\nCategories (article count):",articles.len(),categories.len());
    for (category, count) in &sorted {
        println!(
            "  {category}: {count} ({:.1}%)",
            *count as f64 / articles.len() as f64 * 100.0
        );
    }
    (categories, sorted)
}
pub fn save_categories_to_json(categories: &Map<String, Value>, dir: &Path) -> io::Result<PathBuf> {
    let path = dir.join(format!("categories_{}.json", timestamp()));
    let now = Local::now()
        .naive_local()
        .format("%Y-%m-%dT%H:%M:%S%.6f")
        .to_string();
    let data =
        json!({"analysis_date":now,"total_categories":categories.len(),"categories":categories});
    match write_json(&path, &data) {
        Ok(()) => {
            println!("Categories saved to: {}", path.display());
            Ok(path)
        }
        Err(e) => {
            println!("Error saving categories: {e}");
            Err(e)
        }
    }
}
pub fn get_categories_from_file(root: &Path) -> Vec<(String, u64)> {
    let path = root.join("data/categories.json");
    if !path.exists() {
        return Vec::new();
    }
    match fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
    {
        Some(categories) => {
            println!(
                "\nCategories loaded from file: {}\nAvailable categories ({}):",
                path.display(),
                categories.len()
            );
            for (i, cat) in categories.iter().enumerate() {
                println!("{}. {cat}", i + 1);
            }
            categories.into_iter().map(|c| (c, 0)).collect()
        }
        None => {
            println!("Error loading categories from file: invalid category list");
            Vec::new()
        }
    }
}
pub fn get_categories_from_sample(
    network: &Network,
    latest: i64,
    size: usize,
) -> io::Result<Vec<(String, u64)>> {
    println!("\nRetrieving available categories from a sample of {size} articles...");
    let mut options = ScanOptions::new((latest - size as i64 + 1).max(1), latest);
    options.workers = 5;
    options.batch_size = 25;
    let articles = scan_id_range_parallel_batch(network, &options)?;
    if articles.is_empty() {
        println!("Failed to retrieve a sample of articles for category analysis.");
        return Ok(Vec::new());
    }
    let categories = counts_with_uncategorized(&articles);
    let sorted = sorted_categories(&categories);
    println!("\nAvailable categories (from {} articles):", articles.len());
    for (i, (cat, count)) in sorted.iter().enumerate() {
        println!(
            "{}. {cat}: {count} articles ({:.1}%)",
            i + 1,
            *count as f64 / articles.len() as f64 * 100.0
        );
    }
    Ok(sorted)
}
fn counts_with_uncategorized(articles: &[Value]) -> Map<String, Value> {
    let mut counts = Map::new();
    for a in articles {
        let cat = a
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("Uncategorized");
        let count = counts.get(cat).and_then(Value::as_u64).unwrap_or(0) + 1;
        counts.insert(cat.into(), count.into());
    }
    counts
}
pub fn select_indices(
    choice: &str,
    categories: &[(String, u64)],
) -> Result<Vec<String>, std::num::ParseIntError> {
    if choice.is_empty() {
        return Ok(Vec::new());
    }
    choice
        .split(',')
        .map(|s| s.trim().parse::<i64>())
        .collect::<Result<Vec<_>, _>>()
        .map(|ids| {
            ids.into_iter()
                .filter_map(|i| {
                    if i > 0 {
                        categories.get((i - 1) as usize).map(|p| p.0.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
}
pub fn select_categories_at_start(
    reader: &mut dyn BufRead,
    categories: &[(String, u64)],
) -> io::Result<Vec<String>> {
    if categories.is_empty() {
        return Ok(Vec::new());
    }
    println!("\nCATEGORY SELECTION FOR SCRAPING:\nSelect the categories you want to scrape.\nEnter category numbers separated by commas (e.g., 1,3,5)\nOr enter 'all' for all categories");
    let choice = input(reader, "Your choice: ")?.to_lowercase();
    if choice == "all" {
        return Ok(categories.iter().map(|p| p.0.clone()).collect());
    }
    if choice.is_empty() {
        println!("No categories selected - all will be scraped");
        return Ok(Vec::new());
    }
    match select_indices(&choice, categories) {
        Ok(selected) if !selected.is_empty() => {
            println!("\nSelected categories: {}", selected.join(", "));
            Ok(selected)
        }
        Ok(_) => {
            println!("Invalid category numbers");
            Ok(Vec::new())
        }
        Err(_) => {
            println!("Invalid number format");
            Ok(Vec::new())
        }
    }
}
fn filter_results(
    reader: &mut dyn BufRead,
    articles: &[Value],
    dir: &Path,
    sorted: &[(String, u64)],
) -> io::Result<bool> {
    let choice = input(
        reader,
        "\nEnter category numbers separated by commas (e.g., 1,3,5): ",
    )?;
    if choice.is_empty() {
        return Ok(false);
    }
    match select_indices(&choice, sorted) {
        Ok(selected) if !selected.is_empty() => {
            println!("\nFiltering by categories: {}", selected.join(", "));
            let filename = format!("filtered_content_{}.json", timestamp());
            let filtered = filter_articles_by_categories(articles, &selected);
            save_articles_progressively(&filtered, dir, &filename)?;
            println!(
                "Filtered {} articles out of {}\nFiltered results saved to: {filename}",
                filtered.len(),
                articles.len()
            );
            return Ok(true);
        }
        Ok(_) => println!("Invalid category numbers"),
        Err(_) => println!("Invalid number format"),
    }
    Ok(false)
}
pub fn offer_category_filtering(
    reader: &mut dyn BufRead,
    articles: &[Value],
    dir: &Path,
) -> io::Result<()> {
    if articles.is_empty() {
        return Ok(());
    }
    println!("\nDO YOU WANT TO FILTER BY CATEGORIES?\nFound {} articles. You can filter them by categories.",articles.len());
    if input(reader, "Enter 'y' to filter or Enter to continue: ")?.to_lowercase() == "y" {
        let counts = counts_with_uncategorized(articles);
        let sorted = sorted_categories(&counts);
        println!("\nAvailable categories:");
        for (i, (cat, count)) in sorted.iter().enumerate() {
            println!(
                "{}. {cat}: {count} articles ({:.1}%)",
                i + 1,
                *count as f64 / articles.len() as f64 * 100.0
            );
        }
        if filter_results(reader, articles, dir, &sorted)? {
            save_categories_to_json(&counts, dir)?;
        }
    }
    Ok(())
}
fn menu(latest: i64) {
    println!("🥷 TOR SCRAPING MODE:");
    println!("1. TEST - range {}-{latest} (quick test)", latest - 99);
    println!(
        "2. SMALL - range {}-{latest} (estimate: ~500-800 articles)",
        latest - 999
    );
    println!(
        "3. MEDIUM - range {}-{latest} (estimate: ~5000-8000 articles)",
        latest - 9999
    );
    println!(
        "4. LARGE - range {}-{latest} (estimate: ~25000-40000 articles)",
        latest - 49999
    );
    println!(
        "5. MASSIVE - range {}-{latest} (estimate: ~50000-80000 articles)",
        latest - 99999
    );
    println!(
        "6. MAXIMUM - range 1-{latest} (estimate: ~{latest} articles - scans from ID 1 to newest)"
    );
    println!("7. CUSTOM - enter custom range\n8. CATEGORY ANALYSIS - scrape 200 articles and analyze categories\n\nSCRAPING DIRECTION:\nA. NEWEST → OLDEST (recommended - starts with the newest articles)\nB. OLDEST → NEWEST (starts with the oldest articles)\n\nSAVE FREQUENCY:\n1. Every 25 articles (very frequent)\n2. Every 50 articles (frequent)\n3. Every 100 articles (normal)\n4. Every 200 articles (less frequent)");
}
pub fn run(network: &Network, reader: &mut dyn BufRead, root: &Path) -> io::Result<()> {
    if let Ok(art) = fs::read_to_string(root.join("data/ascii.txt")) {
        println!("{art}\n");
    }
    println!("{}\nChecking Tor connection...", "=".repeat(50));
    if !network.check_tor_connection() {
        println!("Attempting to start Tor service...");
        if !start_tor_service(network) {
            println!("Tor is not available. Please install Tor:\n   macOS: brew install tor && brew services start tor\n   Linux: sudo apt install tor && sudo systemctl start tor\n   Windows: Download Tor Browser");
            return Ok(());
        }
    }
    println!("Tor is ready!\n");
    let latest = network
        .fetch_latest_rss_articles()
        .map(|p| p.0)
        .unwrap_or_else(|| {
            println!("Could not determine latest article ID. Using fallback range.");
            200000
        });
    println!("\nCATEGORY SELECTION:\nDo you want to select specific categories for scraping?");
    let mut selected = Vec::new();
    if input(reader, "Enter 'y' to select categories or Enter for all: ")?.to_lowercase() == "y" {
        let mut categories = get_categories_from_file(root);
        if categories.is_empty() {
            categories = get_categories_from_sample(network, latest, 50)?;
        }
        if categories.is_empty() {
            println!("Failed to retrieve categories. All will be scraped.");
        } else {
            selected = select_categories_at_start(reader, &categories)?;
            if selected.is_empty() {
                println!("All categories will be scraped.");
            }
        }
    } else {
        println!("All categories will be scraped.");
    }
    println!();
    let dir = root.join("output");
    fs::create_dir_all(&dir)?;
    let filename = format!("content_{}.json", timestamp());
    println!(
        "Output directory: {}\nOutput file: {filename}\n",
        dir.display()
    );
    menu(latest);
    let choice = input(reader, "\nEnter choice (1/2/3/4/5/6/7/8): ")?;
    let mut options = ScanOptions::new(1, latest);
    options.output = Some((dir.clone(), filename.clone()));
    options.selected = selected;
    if choice != "8" {
        options.reverse =
            input(reader, "Enter direction (A/B) [A = NEWEST→OLDEST]: ")?.to_uppercase() != "B";
        options.save_frequency =
            match input(reader, "Enter save frequency (1/2/3/4) [3 = every 100]: ")?.as_str() {
                "1" => 25,
                "2" => 50,
                "4" => 200,
                _ => 100,
            };
    }
    match choice.as_str() {
        "1" | "2" | "3" | "4" | "5" | "6" => {
            let (size, workers, batch, label) = match choice.as_str() {
                "1" => (100, 10, 50, "TEST MODE"),
                "2" => (1000, 15, 100, "SMALL DATASET"),
                "3" => (10000, 20, 500, "MEDIUM DATASET"),
                "4" => (50000, 25, 500, "LARGE DATASET"),
                "5" => (100000, 25, 500, "MASSIVE DATASET"),
                _ => (latest, 25, 500, "MAXIMUM DATASET"),
            };
            options.min = (latest - size + 1).max(1);
            options.workers = workers;
            options.batch_size = batch;
            println!("{label}: {}-{latest} (TOR FAST)", options.min);
            if choice == "6" {
                println!("WARNING: This will scan from ID 1 to {latest} ({latest} articles)");
            }
            if choice != "1"
                && choice != "2"
                && input(reader, "Continue? (y/N): ")?.to_lowercase() != "y"
            {
                println!("Cancelled.");
                return Ok(());
            }
        }
        "7" => {
            let min = input(reader, "Enter minimum ID: ")?.parse::<i64>();
            let max = input(reader, "Enter maximum ID: ")?.parse::<i64>();
            let workers = input(reader, "Enter number of workers (recommended 50): ")?;
            let batch = input(reader, "Enter batch size (recommended 1000): ")?;
            let workers = if workers.is_empty() {
                Ok(50)
            } else {
                workers.parse::<usize>()
            };
            let batch = if batch.is_empty() {
                Ok(1000)
            } else {
                batch.parse::<usize>()
            };
            match (min, max, workers, batch) {
                (Ok(min), Ok(max), Ok(workers), Ok(batch)) => {
                    options.min = min;
                    options.max = max;
                    options.workers = workers;
                    options.batch_size = batch;
                }
                _ => {
                    println!("Invalid number!");
                    return Ok(());
                }
            }
            println!("CUSTOM DATASET: {}-{}", options.min, options.max);
            if input(reader, "Continue? (y/N): ")?.to_lowercase() != "y" {
                println!("Cancelled.");
                return Ok(());
            }
        }
        "8" => {
            options.min = (latest - 199).max(1);
            options.workers = 8;
            options.batch_size = 50;
            options.reverse = true;
            options.save_frequency = 50;
            options.selected.clear();
            println!("CATEGORY ANALYSIS: {}-{latest} (200 articles)", options.min);
        }
        _ => {
            println!("Invalid choice!");
            return Ok(());
        }
    }
    let articles = scan_id_range_parallel_batch(network, &options)?;
    if choice == "8" {
        if articles.is_empty() {
            println!("No articles found for category analysis");
        } else {
            let (categories, sorted) = analyze_categories_from_json(&dir.join(&filename));
            if !categories.is_empty() {
                let _ = save_categories_to_json(&categories, &dir);
                println!("\nDO YOU WANT TO FILTER BY CATEGORIES?");
                if input(reader, "Enter 'y' to filter or Enter to continue: ")?.to_lowercase()
                    == "y"
                {
                    println!("\nAvailable categories:");
                    for (i, (cat, count)) in sorted.iter().enumerate() {
                        println!("{}. {cat} ({count} articles)", i + 1);
                    }
                    filter_results(reader, &articles, &dir, &sorted)?;
                }
            }
        }
    }
    if articles.is_empty() {
        println!("\nNo articles found.");
    } else {
        println!(
            "\nSCRAPING COMPLETE!\nTotal articles found: {}\nSaved to: {filename}\nLocation: {}",
            articles.len(),
            dir.display()
        );
        match fs::metadata(dir.join(&filename)) {
            Ok(m) => println!("File size: {:.1} MB", m.len() as f64 / 1024.0 / 1024.0),
            Err(_) => println!("File size: Calculating..."),
        }
    }
    Ok(())
}
