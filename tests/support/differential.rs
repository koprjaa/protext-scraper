// Project: protext-scraper
// File: tests/support/differential.rs
// Description: Replay captured Python reference inputs without a Python runtime.
// Author: Jan Alexandr Kopřiva
// License: MIT
use protext_scraper::{network::decode, parsing::*, scanning::*, storage::*};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
pub fn evaluate(c: &Value) -> Value {
    let s = c["input"].as_str().unwrap_or("");
    match c["op"].as_str().unwrap() {
        "large" => {
            let source = format!(
                "<div class='fulltext'>{}</div>",
                "ř".repeat(c["input"].as_u64().unwrap() as usize)
            );
            json!(extract_content(&parse_html(&source)).chars().count())
        }
        "write_directory" => {
            let d = tempfile::tempdir().unwrap();
            json!({"error":write_articles(d.path(),&[json!({"id":1})]).is_err()})
        }
        "clean" => json!(clean_content(s)),
        "keywords" => json!(clean_keywords(s)),
        "id" => json!(extract_protext_id(s)),
        "article" => json!(parse_article(s, 7)),
        "content" => json!(extract_content(&scraper::Html::parse_document(s))),
        "category" => json!(extract_category(&scraper::Html::parse_document(s))),
        "rss" => json!(rss_ids(s).ok()),
        "decode" => json!(decode(
            &c["input"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap() as u8)
                .collect::<Vec<_>>()
        )),
        "dedup" => json!(deduplicate(c["input"].as_array().unwrap())),
        "counts" => json!(count_categories(c["input"].as_array().unwrap())),
        "load" => {
            let d = tempfile::tempdir().unwrap();
            let path = d.path().join("in.json");
            if let Some(bytes) = c["input"].as_array() {
                std::fs::write(
                    &path,
                    bytes
                        .iter()
                        .map(|v| v.as_u64().unwrap() as u8)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            }
            json!(load_articles(&path))
        }
        "select" => match protext_scraper::app::select_indices(
            s,
            &[("Sport".into(), 0), ("Economy".into(), 0)],
        ) {
            Ok(v) => json!(v),
            Err(_) => json!({"error":"ValueError"}),
        },
        "request" => {
            let (n, m) = crate::support::network();
            let i = &c["input"];
            *m.statuses.lock().unwrap() = i["statuses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| match v.as_u64() {
                    Some(n) => Ok(n as u16),
                    None => Err(v.as_str().unwrap().into()),
                })
                .collect();
            *m.retry_after.lock().unwrap() = i["retry_after"].as_str().map(String::from);
            let r = n.request(
                "http://fixture.test",
                i["retries"].as_u64().unwrap() as usize,
                1.0,
                i["tor"].as_bool().unwrap(),
            );
            json!({"status":r.map(|r|r.status),"sleeps":*m.sleeps.lock().unwrap(),"renewals":m.renewals.load(Ordering::SeqCst),"requests":m.requests.lock().unwrap().len()})
        }
        "scan" => {
            let (n, _) = crate::support::network();
            let d = tempfile::tempdir().unwrap();
            let i = &c["input"];
            let mut o = ScanOptions::new(i["min"].as_i64().unwrap(), i["max"].as_i64().unwrap());
            o.batch_size = i["batch"].as_u64().unwrap() as usize;
            o.step = i["step"].as_u64().unwrap() as usize;
            o.reverse = i["reverse"].as_bool().unwrap();
            o.workers = 1;
            o.selected = i["selected"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().into())
                .collect();
            if i["output"] == true {
                o.output = Some((d.path().into(), "out.json".into()));
            }
            let rows = scan_id_range_parallel_batch(&n, &o).unwrap();
            let path = d.path().join("out.json");
            let file = path.exists().then(|| {
                json!(load_articles(&path)
                    .iter()
                    .map(|a| a["id"].clone())
                    .collect::<Vec<_>>())
            });
            json!({"ids":rows.iter().map(|a|a["id"].clone()).collect::<Vec<_>>(),"file":file})
        }
        _ => panic!("Unknown reference operation"),
    }
}
