// Project: protext-scraper
// File: storage.rs
// Description: Locked JSON persistence and category utilities.
// Author: Jan Alexandr Kopřiva
// License: MIT

use crate::parsing::trim;
use serde_json::{Map, Value};
use std::{collections::HashSet, fs, io, path::Path, sync::Mutex};
pub static FILE_LOCK: Mutex<()> = Mutex::new(());
pub fn load_articles(path: &Path) -> Vec<Value> {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
}
pub fn write_json(path: &Path, value: &Value) -> io::Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)
}
pub fn write_articles(path: &Path, articles: &[Value]) -> io::Result<()> {
    write_json(path, &Value::Array(articles.to_vec()))
}
fn id_key(value: &Value) -> String {
    // Python's set considers True, 1, and 1.0 equal, but keeps "1" separate.
    match value {
        Value::Bool(b) => format!("number:{}", u8::from(*b)),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                return format!("number:{n}");
            }
            let number = n.as_f64().unwrap();
            if number == 0.0 {
                return "number:0".into();
            }
            if number.fract() == 0.0 {
                return format!("number:{number:.0}");
            }
            format!("number:{n}")
        }
        _ => format!("json:{value}"),
    }
}
pub fn deduplicate(articles: &[Value]) -> (Vec<Value>, usize) {
    let mut seen = HashSet::new();
    let mut kept = Vec::new();
    for article in articles {
        let id = article.get("id").unwrap_or(&Value::Null);
        if !id.is_null() && !seen.insert(id_key(id)) {
            continue;
        }
        kept.push(article.clone());
    }
    let dropped = articles.len() - kept.len();
    (kept, dropped)
}
pub fn remove_duplicates_from_json(path: &Path) -> io::Result<Option<Vec<Value>>> {
    let _lock = FILE_LOCK.lock().unwrap();
    let articles = load_articles(path);
    if articles.is_empty() {
        return Ok(None);
    }
    let (cleaned, dropped) = deduplicate(&articles);
    write_articles(path, &cleaned)?;
    println!(
        "Removed {dropped} duplicate articles from {}",
        path.display()
    );
    println!(
        "Original: {} articles, Cleaned: {} articles",
        articles.len(),
        cleaned.len()
    );
    Ok(Some(cleaned))
}
pub fn save_articles_progressively(
    articles: &[Value],
    dir: &Path,
    filename: &str,
) -> io::Result<usize> {
    if articles.is_empty() {
        return Ok(0);
    }
    let _lock = FILE_LOCK.lock().unwrap();
    let path = dir.join(filename);
    let mut existing = load_articles(&path);
    let mut ids = existing
        .iter()
        .filter_map(|a| a.get("id"))
        .filter(|id| !id.is_null())
        .map(id_key)
        .collect::<HashSet<_>>();
    let mut added = 0;
    for article in articles {
        let id = article.get("id").unwrap_or(&Value::Null);
        if !id.is_null() && !ids.insert(id_key(id)) {
            continue;
        }
        added += 1;
        existing.push(article.clone());
    }
    fs::create_dir_all(dir)?;
    write_articles(&path, &existing)?;
    println!(
        "Saved {added} new articles to {filename} (Skipped {} duplicates, Total: {})",
        articles.len() - added,
        existing.len()
    );
    Ok(added)
}
pub fn count_categories(articles: &[Value]) -> Map<String, Value> {
    let mut counts = Map::new();
    for article in articles {
        let cat = trim(
            article
                .get("category")
                .and_then(Value::as_str)
                .unwrap_or(""),
        );
        if !cat.is_empty() {
            let n = counts.get(cat).and_then(Value::as_u64).unwrap_or(0) + 1;
            counts.insert(cat.into(), n.into());
        }
    }
    counts
}
pub fn sorted_categories(counts: &Map<String, Value>) -> Vec<(String, u64)> {
    let mut result = counts
        .iter()
        .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
        .collect::<Vec<_>>();
    result.sort_by_key(|a| std::cmp::Reverse(a.1));
    result
}
pub fn filter_articles_by_categories(articles: &[Value], selected: &[String]) -> Vec<Value> {
    articles
        .iter()
        .filter(|a| {
            selected.is_empty()
                || selected
                    .iter()
                    .any(|s| s == trim(a.get("category").and_then(Value::as_str).unwrap_or("")))
        })
        .cloned()
        .collect()
}
