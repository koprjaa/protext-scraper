// Project: protext-scraper
// File: tests/storage.rs
// Description: Equivalents of every original storage test and concurrent-write checks.
// Author: Jan Alexandr Kopřiva
// License: MIT
use protext_scraper::storage::*;
use serde_json::{json, Value};
use std::fs;
fn article(id: i64) -> Value {
    json!({"id":id,"title":format!("Zpráva {id}")})
}
fn ids(a: &[Value]) -> Vec<i64> {
    a.iter().map(|a| a["id"].as_i64().unwrap()).collect()
}
#[test]
fn test_a_missing_file_reads_as_empty() {
    let d = tempfile::tempdir().unwrap();
    assert!(load_articles(&d.path().join("nope.json")).is_empty());
}
#[test]
fn test_a_truncated_file_reads_as_empty() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out.json");
    fs::write(&p, "[{\"id\":1},{\"id\":").unwrap();
    assert!(load_articles(&p).is_empty());
}
#[test]
fn test_a_file_holding_something_other_than_a_list_reads_as_empty() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out.json");
    fs::write(&p, "{\"id\":1}").unwrap();
    assert!(load_articles(&p).is_empty());
}
#[test]
fn test_a_repeated_id_is_dropped_once() {
    let (a, n) = deduplicate(&[article(1), article(2), article(1)]);
    assert_eq!(ids(&a), [1, 2]);
    assert_eq!(n, 1);
}
#[test]
fn test_the_first_copy_of_an_id_is_the_one_kept() {
    let (a, _) = deduplicate(&[
        json!({"id":1,"title":"první"}),
        json!({"id":1,"title":"druhý"}),
    ]);
    assert_eq!(a[0]["title"], "první");
}
#[test]
fn test_an_article_without_an_id_is_kept() {
    let (a, n) = deduplicate(&[json!({"title":"bez id"}), json!({"title":"taky bez id"})]);
    assert_eq!(a.len(), 2);
    assert_eq!(n, 0);
}
#[test]
fn test_deduplicating_an_empty_list() {
    assert_eq!(deduplicate(&[]), (Vec::new(), 0));
}
#[test]
fn test_the_results_file_is_rewritten_without_duplicates() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out.json");
    write_articles(&p, &[article(1), article(2), article(1)]).unwrap();
    remove_duplicates_from_json(&p).unwrap();
    assert_eq!(ids(&load_articles(&p)), [1, 2]);
}
#[test]
fn test_cleaning_a_missing_file_gives_none() {
    let d = tempfile::tempdir().unwrap();
    assert!(remove_duplicates_from_json(&d.path().join("nope.json"))
        .unwrap()
        .is_none());
}
#[test]
fn test_articles_are_written() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        save_articles_progressively(&[article(1), article(2)], d.path(), "out.json").unwrap(),
        2
    );
    assert_eq!(ids(&load_articles(&d.path().join("out.json"))), [1, 2]);
}
#[test]
fn test_a_second_batch_appends() {
    let d = tempfile::tempdir().unwrap();
    save_articles_progressively(&[article(1)], d.path(), "out.json").unwrap();
    save_articles_progressively(&[article(2)], d.path(), "out.json").unwrap();
    assert_eq!(ids(&load_articles(&d.path().join("out.json"))), [1, 2]);
}
#[test]
fn test_an_article_already_on_disk_is_not_written_again() {
    let d = tempfile::tempdir().unwrap();
    save_articles_progressively(&[article(1)], d.path(), "out.json").unwrap();
    assert_eq!(
        save_articles_progressively(&[article(1), article(2)], d.path(), "out.json").unwrap(),
        1
    );
    assert_eq!(ids(&load_articles(&d.path().join("out.json"))), [1, 2]);
}
#[test]
fn test_a_repeat_inside_one_batch_is_written_once() {
    let d = tempfile::tempdir().unwrap();
    save_articles_progressively(&[article(1), article(1)], d.path(), "out.json").unwrap();
    assert_eq!(ids(&load_articles(&d.path().join("out.json"))), [1]);
}
#[test]
fn test_an_article_without_an_id_is_written() {
    let d = tempfile::tempdir().unwrap();
    save_articles_progressively(&[json!({"title":"bez id"})], d.path(), "out.json").unwrap();
    assert_eq!(load_articles(&d.path().join("out.json")).len(), 1);
}
#[test]
fn test_an_empty_batch_writes_nothing() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        save_articles_progressively(&[], d.path(), "out.json").unwrap(),
        0
    );
    assert!(!d.path().join("out.json").exists());
}
#[test]
fn test_a_missing_output_directory_is_created() {
    let d = tempfile::tempdir().unwrap();
    let target = d.path().join("nested/run");
    save_articles_progressively(&[article(1)], &target, "out.json").unwrap();
    assert!(target.join("out.json").exists());
}
#[test]
fn test_writing_over_a_broken_file_does_not_lose_the_new_batch() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("out.json"), "not json at all").unwrap();
    save_articles_progressively(&[article(1)], d.path(), "out.json").unwrap();
    assert_eq!(ids(&load_articles(&d.path().join("out.json"))), [1]);
}
#[test]
fn test_categories_are_counted() {
    let a = [
        json!({"category":"Ekonomika"}),
        json!({"category":"Ekonomika"}),
        json!({"category":"Sport"}),
    ];
    assert_eq!(
        Value::Object(count_categories(&a)),
        json!({"Ekonomika":2,"Sport":1})
    );
}
#[test]
fn test_articles_without_a_category_are_not_counted() {
    assert!(count_categories(&[article(1), json!({"category":"  "})]).is_empty());
}
#[test]
fn test_filtering_keeps_only_the_selected_categories() {
    let a = [
        json!({"id":1,"category":"Ekonomika"}),
        json!({"id":2,"category":"Sport"}),
    ];
    assert_eq!(
        ids(&filter_articles_by_categories(&a, &["Sport".into()])),
        [2]
    );
}
#[test]
fn test_filtering_with_no_selection_keeps_everything() {
    assert_eq!(
        filter_articles_by_categories(&[article(1), article(2)], &[]).len(),
        2
    );
}
#[test]
fn test_filtering_matches_a_category_with_stray_whitespace() {
    assert_eq!(
        filter_articles_by_categories(
            &[json!({"category":"  Ekonomika  "})],
            &["Ekonomika".into()]
        )
        .len(),
        1
    );
}
#[test]
fn concurrent_appends_and_null_ids() {
    let d = tempfile::tempdir().unwrap();
    std::thread::scope(|s| {
        for id in 0..12 {
            let p = d.path();
            s.spawn(move || {
                save_articles_progressively(&[article(id), json!({"id":null})], p, "out.json")
                    .unwrap()
            });
        }
    });
    let a = load_articles(&d.path().join("out.json"));
    assert_eq!(a.len(), 24);
    assert_eq!(deduplicate(&a).1, 0);
}
#[test]
fn json_bytes_match_original_format() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out.json");
    write_articles(&p, &[json!({"id":1,"title":"Zpráva"})]).unwrap();
    assert_eq!(
        fs::read(p).unwrap(),
        b"[\n  {\n    \"id\": 1,\n    \"title\": \"Zpr\xc3\xa1va\"\n  }\n]"
    );
}

#[test]
fn numeric_ids_follow_python_set_equality() {
    let articles = [
        json!({"id":true}),
        json!({"id":1}),
        json!({"id":1.0}),
        json!({"id":"1"}),
        json!({"id":false}),
        json!({"id":0}),
        json!({"id":-0.0}),
        json!({"id":null}),
        json!({"id":null}),
    ];
    let (kept, dropped) = deduplicate(&articles);
    assert_eq!(dropped, 4);
    assert_eq!(kept.len(), 5);
    let d = tempfile::tempdir().unwrap();
    assert_eq!(
        save_articles_progressively(&articles, d.path(), "out.json").unwrap(),
        5
    );
}
#[test]
fn test_saving_sets_a_corrupt_results_file_aside_instead_of_overwriting_it() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out.json");
    fs::write(&p, "[{\"id\":1},{\"id\":").unwrap();
    save_articles_progressively(&[article(2)], d.path(), "out.json").unwrap();
    assert_eq!(ids(&load_articles(&p)), [2]);
    assert_eq!(
        fs::read_to_string(d.path().join("out.json.corrupt")).unwrap(),
        "[{\"id\":1},{\"id\":"
    );
}
#[test]
fn test_a_write_failure_keeps_the_previous_results_and_leaves_no_temp_file() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("out.json");
    save_articles_progressively(&[article(1)], d.path(), "out.json").unwrap();
    // A directory in the way of the rename makes the final step fail after the temp file is written.
    let blocked = d.path().join("blocked.json");
    fs::create_dir(&blocked).unwrap();
    assert!(write_articles(&blocked, &[article(9)]).is_err());
    assert_eq!(ids(&load_articles(&p)), [1]);
    assert!(!d.path().join("blocked.json.tmp").exists());
}
