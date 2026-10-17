// Project: protext-scraper
// File: tests/parsing.rs
// Description: Equivalents of every original parser test and parser regressions.
// Author: Jan Alexandr Kopřiva
// Based on protext-scraper by Jan Alexandr Kopřiva
// License: MIT
use protext_scraper::parsing::*;
use scraper::Html;
const ARTICLE: &str = include_str!("fixtures/article.html");
fn page() -> Html {
    Html::parse_document(ARTICLE)
}
fn long() -> String {
    format!("Firma oznámila nový produkt na trhu{}", "x".repeat(16))
}
#[test]
fn test_html_tags_are_removed() {
    let b = long();
    assert_eq!(clean_content(&format!("<p>{b}</p>")), b);
}
#[test]
fn test_cdata_wrapper_is_removed() {
    let b = long();
    assert_eq!(clean_content(&format!("<![CDATA[{b}]]>")), b);
}
#[test]
fn test_whitespace_is_collapsed() {
    assert_eq!(
        clean_content("  Firma\n\n  oznámila   nový   produkt na českém trhu už dnes ráno  "),
        "Firma oznámila nový produkt na českém trhu už dnes ráno"
    );
}
#[test]
fn test_content_below_the_minimum_is_dropped() {
    for s in ["", "Krátké", &"x".repeat(49)] {
        assert_eq!(clean_content(s), "");
    } /* None is represented by the caller's absent Option. */
}
#[test]
fn test_content_at_the_minimum_is_kept() {
    assert_eq!(clean_content(&"x".repeat(50)), "x".repeat(50));
}
#[test]
fn test_a_page_that_is_only_markup_is_dropped() {
    assert_eq!(clean_content("<div><span></span></div>"), "");
}
#[test]
fn test_dashes_around_the_keyword_list_are_stripped() {
    assert_eq!(
        clean_keywords("- ekonomika, finance -"),
        "ekonomika, finance"
    );
}
#[test]
fn test_every_dash_character_is_stripped() {
    for d in ["-", "–", "—"] {
        assert_eq!(clean_keywords(&format!("{d} ekonomika {d}")), "ekonomika");
    }
}
#[test]
fn test_keywords_with_nothing_behind_the_label_are_dropped() {
    for s in ["", "  ", "- -", "ab"] {
        assert_eq!(clean_keywords(s), "");
    }
}
#[test]
fn test_the_id_is_read_from_the_url() {
    assert_eq!(
        extract_protext_id("https://www.protext.cz/zprava.php?id=53986"),
        Some(53986)
    );
}
#[test]
fn test_the_id_is_found_among_other_parameters() {
    assert_eq!(
        extract_protext_id("https://www.protext.cz/zprava.php?lang=cs&id=1&x=2"),
        Some(1)
    );
}
#[test]
fn test_a_url_without_a_protext_id_gives_none() {
    for s in [
        "",
        "https://example.com/zprava.php?id=1",
        "https://www.protext.cz/index.php",
    ] {
        assert_eq!(extract_protext_id(s), None);
    }
}
#[test]
fn test_the_body_comes_from_its_own_container() {
    assert!(extract_content(&page()).contains("Tělo článku"));
}
#[test]
fn test_the_navigation_is_left_out_of_the_body() {
    assert!(!extract_content(&page()).contains("Přejít k obsahu"));
}
#[test]
fn test_a_page_without_the_container_gives_empty() {
    assert_eq!(
        extract_content(&Html::parse_document(
            "<html><body><p>Nic</p></body></html>"
        )),
        ""
    );
}
#[test]
fn test_the_keywords_are_read_from_the_metadata_box() {
    assert_eq!(extract_keywords(&page()), "ČR-Korea-víra-právo");
}
#[test]
fn test_the_keyword_label_is_not_part_of_the_result() {
    assert!(!extract_keywords(&page()).contains("Klíčová slova"));
}
#[test]
fn test_a_page_with_no_keywords_gives_empty() {
    assert_eq!(
        extract_keywords(&Html::parse_document(
            "<html><body><p>Text</p></body></html>"
        )),
        ""
    );
}
#[test]
fn test_a_metadata_box_without_a_keyword_paragraph_gives_empty() {
    assert_eq!(extract_keywords(&Html::parse_document("<div class='fulltext-metadata'><p><strong>Kategorie</strong><br><span>Sport</span></p></div>")),"");
}
#[test]
fn test_every_category_is_returned_not_only_the_first() {
    assert_eq!(
        extract_category(&page()),
        "Náboženství, Politika, veřejná správa"
    );
}
#[test]
fn test_the_category_label_is_not_part_of_the_result() {
    assert!(!extract_category(&page()).contains("Kategorie"));
}
#[test]
fn test_a_single_category_comes_back_on_its_own() {
    assert_eq!(extract_category(&Html::parse_document("<div class='fulltext-metadata'><p><strong>Kategorie</strong><br><span>Sport</span></p></div>")),"Sport");
}
#[test]
fn test_a_page_with_no_category_gives_empty() {
    assert_eq!(
        extract_category(&Html::parse_document("<html><body></body></html>")),
        ""
    );
}
#[test]
fn path_ids_and_query_order() {
    for path in ["zprava/58633", "zpravy/show/58633"] {
        assert_eq!(
            extract_protext_id(&format!(
                "https://www.protext.cz/{path}?campaign_id=99&id=7"
            )),
            Some(58633)
        );
    }
    assert_eq!(extract_protext_id("https://protext.cz/?id=12x"), None);
}
#[test]
fn unicode_length_and_html_entities() {
    assert_eq!(clean_content(&"ř".repeat(49)), "");
    assert_eq!(clean_content(&"ř".repeat(50)), "ř".repeat(50));
    let html = format!(
        "<h1>{}</h1><div class='fulltext'>a &amp; b {}</div>",
        "ž".repeat(50),
        "x".repeat(60)
    );
    let a = parse_article(&html, 12).unwrap();
    assert!(a["content"].as_str().unwrap().starts_with("a & b"));
    assert_eq!(a["id"], 12);
}
#[test]
fn short_title_is_kept_and_field_order_preserved() {
    assert_eq!(
        parse_article(&format!("<h1>short</h1>{ARTICLE}"), 1).unwrap()["title"],
        "short"
    );
    assert!(parse_article(&format!("<h1>  </h1>{ARTICLE}"), 1).is_none());
    let a = parse_article(
        &format!("<h1>{}</h1><time> today </time>{ARTICLE}", "x".repeat(50)),
        1,
    )
    .unwrap();
    assert_eq!(
        a.as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["title", "content", "link", "id", "date", "keywords", "category"]
    );
}
#[test]
fn rss_links_and_cdata() {
    let ids=rss_ids("<rss><channel><item><link>https://protext.cz/zprava/4</link></item><item><link><![CDATA[https://protext.cz/zprava.php?id=9&x=1]]></link></item><item><link>https://example.com/?id=99</link></item></channel></rss>").unwrap();
    assert_eq!(ids, [4, 9]);
}
#[test]
fn full_content_selectors_removal_and_threshold() {
    let p=format!("<nav>navigation</nav><article>{}</article><article>{}</article><div class='content'>{}</div>","a".repeat(101),"b".repeat(120),"c".repeat(200));
    assert_eq!(full_content(&p), Some("b".repeat(120)));
    assert_eq!(full_content(&"x".repeat(100)), None);
    assert_eq!(
        full_content(&format!(
            "<script>{}</script><p>{}</p>",
            "z".repeat(200),
            "x".repeat(101)
        )),
        Some("x".repeat(101))
    );
}
