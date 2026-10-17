// Project: protext-scraper
// File: tests/workflow.rs
// Description: Offline interactive workflows and scanner regressions.
// Author: Jan Alexandr Kopřiva
// Based on protext-scraper by Jan Alexandr Kopřiva
// License: MIT
mod support;
use protext_scraper::{app::*, scanning::*, storage::*};
use serde_json::json;
use std::io::Cursor;
#[test]
fn every_menu_mode_and_category_analysis() {
    for choice in 1..=8 {
        let (n, m) = support::network();
        let d = tempfile::tempdir().unwrap();
        let script = match choice {
            1 | 2 => format!("\n{choice}\n\n\n"),
            3..=6 => format!("\n{choice}\n\n\ny\n"),
            7 => "\n7\nB\n1\n1\n3\n1\n2\ny\n".into(),
            _ => "\n8\ny\n1\n".into(),
        };
        run(&n, &mut Cursor::new(script), d.path()).unwrap();
        let files = std::fs::read_dir(d.path().join("output"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        let content = files
            .iter()
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("content_")
            })
            .unwrap();
        assert_eq!(load_articles(content).len(), 3, "mode {choice}");
        if choice == 8 {
            assert_eq!(files.len(), 3);
        }
        assert!(m
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|r| r.0.starts_with("https://")));
    }
}
#[test]
fn cancellation_invalid_choice_and_eof() {
    for script in [
        "\n3\n\n\nn\n",
        "\ninvalid\n\n\n",
        "\n7\n\n\ninvalid\n3\n\n\n",
    ] {
        let (n, _) = support::network();
        let d = tempfile::tempdir().unwrap();
        run(&n, &mut Cursor::new(script), d.path()).unwrap();
        assert_eq!(
            std::fs::read_dir(d.path().join("output")).unwrap().count(),
            0
        );
    }
    let (n, _) = support::network();
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        run(&n, &mut Cursor::new(""), d.path()).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
#[test]
fn custom_direction_and_save_frequency_are_honored() {
    let (n, m) = support::network();
    let d = tempfile::tempdir().unwrap();
    run(&n, &mut Cursor::new("\n7\nB\n1\n1\n3\n1\n2\ny\n"), d.path()).unwrap();
    let req = m.requests.lock().unwrap();
    let ids = req
        .iter()
        .filter(|r| r.0.contains("/show/"))
        .map(|r| r.0.rsplit('/').next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["1", "2", "3"]);
}
#[test]
fn sample_without_output_then_real_scan_fetches_ids_again() {
    let (n, m) = support::network();
    assert!(!get_categories_from_sample(&n, 3, 3).unwrap().is_empty());
    let o = ScanOptions::new(1, 3);
    assert_eq!(scan_id_range_parallel_batch(&n, &o).unwrap().len(), 3);
    assert_eq!(m.requests.lock().unwrap().len(), 6);
}
#[test]
fn bounded_concurrency_and_batch_saving() {
    let (n, m) = support::network();
    let d = tempfile::tempdir().unwrap();
    let mut o = ScanOptions::new(1, 20);
    o.workers = 3;
    o.batch_size = 7;
    o.save_frequency = 5;
    o.output = Some((d.path().into(), "out.json".into()));
    assert_eq!(scan_id_range_parallel_batch(&n, &o).unwrap().len(), 20);
    assert_eq!(load_articles(&d.path().join("out.json")).len(), 20);
    assert!(m.peak.load(std::sync::atomic::Ordering::SeqCst) <= 3);
    assert!(m.peak.load(std::sync::atomic::Ordering::SeqCst) > 1);
}
#[test]
fn invalid_ranges_and_sizes_are_rejected() {
    let (n, _) = support::network();
    for which in 0..5 {
        let mut o = ScanOptions::new(1, 3);
        match which {
            0 => o.min = 4,
            1 => o.workers = 0,
            2 => o.batch_size = 0,
            3 => o.step = 0,
            _ => o.save_frequency = 0,
        }
        assert!(scan_id_range_parallel_batch(&n, &o).is_err());
    }
}
#[test]
fn category_selection_all_invalid_and_missing_cache() {
    let cats = vec![("Sport".into(), 2), ("Economy".into(), 1)];
    for (s, n) in [
        ("all\n", 2),
        ("1,2,99,0\n", 2),
        ("abc\n", 0),
        ("99\n", 0),
        ("\n", 0),
    ] {
        assert_eq!(
            select_categories_at_start(&mut Cursor::new(s), &cats)
                .unwrap()
                .len(),
            n
        );
    }
    let d = tempfile::tempdir().unwrap();
    assert!(get_categories_from_file(d.path()).is_empty());
    std::fs::create_dir(d.path().join("data")).unwrap();
    std::fs::write(
        d.path().join("data/categories.json"),
        "[\"Sport\",\"Economy\"]",
    )
    .unwrap();
    assert_eq!(
        get_categories_from_file(d.path()),
        vec![("Sport".into(), 0), ("Economy".into(), 0)]
    );
}
#[test]
fn category_filtering_and_analysis_report() {
    let d = tempfile::tempdir().unwrap();
    let articles = vec![
        json!({"id":1,"category":"Sport"}),
        json!({"id":2,"category":"Economy"}),
    ];
    write_articles(&d.path().join("out.json"), &articles).unwrap();
    let (counts, sorted) = analyze_categories_from_json(&d.path().join("out.json"));
    assert_eq!(sorted[0].0, "Sport");
    let p = save_categories_to_json(&counts, d.path()).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    assert_eq!(v["total_categories"], 2);
    assert!(v["analysis_date"].as_str().unwrap().contains('T'));
    offer_category_filtering(&mut Cursor::new("y\n1\n"), &articles, d.path()).unwrap();
    let filtered = std::fs::read_dir(d.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("filtered_")
        })
        .unwrap();
    assert_eq!(load_articles(&filtered), vec![articles[0].clone()]);
}
#[test]
fn scans_preserve_prior_results_and_empty_results_are_valid_json() {
    let (n, _) = support::network();
    let d = tempfile::tempdir().unwrap();
    write_articles(&d.path().join("out.json"), &[json!({"id":99})]).unwrap();
    let mut o = ScanOptions::new(1, 1);
    o.output = Some((d.path().into(), "out.json".into()));
    scan_id_range_parallel_batch(&n, &o).unwrap();
    assert_eq!(load_articles(&d.path().join("out.json")).len(), 2);
    o.output = Some((d.path().into(), "empty.json".into()));
    o.selected = vec!["not selected".into()];
    assert!(scan_id_range_parallel_batch(&n, &o).unwrap().is_empty());
    assert_eq!(std::fs::read(d.path().join("empty.json")).unwrap(), b"[]");
}
#[test]
fn sequential_and_nonbatch_helpers() {
    let (n, _) = support::network();
    let d = tempfile::tempdir().unwrap();
    let mut o = ScanOptions::new(1, 5);
    o.output = Some((d.path().into(), "out.json".into()));
    assert_eq!(scan_id_range(&n, &o).unwrap().len(), 5);
    assert_eq!(load_articles(&d.path().join("out.json")).len(), 5);
    assert_eq!(scan_id_range_parallel(&n, &o).unwrap().len(), 5);
}
#[test]
fn earlier_reports_survive_new_run() {
    let (n, _) = support::network();
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("output")).unwrap();
    let path = d.path().join("output/content_20000101_000000.json");
    std::fs::write(&path, "[]").unwrap();
    run(&n, &mut Cursor::new("\ninvalid\n\n\n"), d.path()).unwrap();
    assert!(path.exists());
}

#[test]
fn periodic_renewal_and_step_ranges() {
    let (n, m) = support::network();
    let mut o = ScanOptions::new(1, 17);
    o.batch_size = 1;
    o.workers = 1;
    scan_id_range_parallel_batch(&n, &o).unwrap();
    assert_eq!(m.renewals.load(std::sync::atomic::Ordering::SeqCst), 1);
    let (n, m) = support::network();
    let mut o = ScanOptions::new(1, 8);
    o.step = 2;
    o.batch_size = 4;
    o.workers = 1;
    let ids = scan_id_range_parallel_batch(&n, &o)
        .unwrap()
        .into_iter()
        .map(|a| a["id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, [8, 6, 4, 2]);
    assert_eq!(m.requests.lock().unwrap().len(), 4);
}

#[test]
fn invalid_filter_selection_does_not_write_reports() {
    let d = tempfile::tempdir().unwrap();
    let articles = [json!({"id":1,"category":"Sport"})];
    offer_category_filtering(&mut Cursor::new("y\ninvalid\n"), &articles, d.path()).unwrap();
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
}
