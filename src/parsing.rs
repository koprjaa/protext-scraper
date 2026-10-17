// Project: protext-scraper
// File: parsing.rs
// Description: Extract fields from Protext article pages.
// Author: Jan Alexandr Kopřiva
// Based on protext-scraper by Jan Alexandr Kopřiva
// License: MIT

use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub const MIN_CONTENT_LENGTH: usize = 50;
static CDATA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!\[CDATA\[(.*?)\]\]>").unwrap());
static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static IDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/zprav(?:a|y/show)/(\d+)\b|[?&]id=(\d+)\b").unwrap());

pub fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn clean_markup(text: &str) -> String {
    let unwrapped = CDATA.replace_all(text, "$1");
    normalize(&TAG.replace_all(&unwrapped, ""))
}
pub fn clean_content(text: &str) -> String {
    let text = clean_markup(text);
    if text.chars().count() < MIN_CONTENT_LENGTH {
        String::new()
    } else {
        text
    }
}
pub fn clean_keywords(text: &str) -> String {
    let text = normalize(text);
    let text = text.trim_matches(|c: char| c.is_whitespace() || "-–—".contains(c));
    if text.chars().count() > 2 {
        text.into()
    } else {
        String::new()
    }
}
pub fn extract_protext_id(url: &str) -> Option<i64> {
    if !url.contains("protext.cz") {
        return None;
    }
    let m = IDS.captures(url)?;
    m.get(1).or_else(|| m.get(2))?.as_str().parse().ok()
}
pub fn first<'a>(html: &'a Html, selector: &str) -> Option<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).next()
}
pub fn text(element: ElementRef<'_>, separator: &str, strip: bool) -> String {
    // BeautifulSoup excludes script/style strings from ordinary get_text calls.
    element
        .descendants()
        .filter_map(|node| {
            let value = node.value().as_text()?;
            if node
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|p| matches!(p.value().name(), "script" | "style"))
            {
                return None;
            }
            let s: &str = value;
            let s = if strip { s.trim() } else { s };
            (!strip || !s.is_empty()).then_some(s)
        })
        .collect::<Vec<_>>()
        .join(separator)
}
fn metadata<'a>(html: &'a Html, label: &str) -> Option<ElementRef<'a>> {
    let box_element = first(html, "div.fulltext-metadata")?;
    box_element
        .select(&Selector::parse("p").unwrap())
        .find(|p| {
            p.select(&Selector::parse("strong").unwrap())
                .next()
                .is_some_and(|s| text(s, "", false).contains(label))
        })
}
pub fn extract_content(html: &Html) -> String {
    first(html, "div.fulltext")
        .map(|e| clean_content(&text(e, " ", true)))
        .unwrap_or_default()
}
pub fn extract_keywords(html: &Html) -> String {
    metadata(html, "Klíčová slova")
        .map(|p| clean_keywords(&text(p, " ", true).replacen("Klíčová slova", "", 1)))
        .unwrap_or_default()
}
pub fn extract_category(html: &Html) -> String {
    metadata(html, "Kategorie")
        .map(|p| {
            p.select(&Selector::parse("span").unwrap())
                .map(|s| text(s, "", true))
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}
pub fn parse_article(content: &str, id: i64) -> Option<Value> {
    let html = Html::parse_document(content);
    let mut data = Map::new();
    if let Some(e) = first(&html, "h1[itemprop='name headline']")
        .or_else(|| first(&html, "h1"))
        .or_else(|| first(&html, "title"))
    {
        data.insert("title".into(), clean_markup(&text(e, "", false)).into());
    }
    data.insert("content".into(), extract_content(&html).into());
    data.insert(
        "link".into(),
        format!("https://www.protext.cz/zpravy/show/{id}").into(),
    );
    data.insert("id".into(), id.into());
    if let Some(e) = first(&html, "p[itemprop=datePublished]")
        .or_else(|| first(&html, "time"))
        .or_else(|| first(&html, ".date"))
    {
        data.insert("date".into(), text(e, "", false).trim().into());
    }
    for (key, value) in [
        ("keywords", extract_keywords(&html)),
        ("category", extract_category(&html)),
    ] {
        if !value.is_empty() {
            data.insert(key.into(), value.into());
        }
    }
    (data
        .get("title")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.is_empty())
        && data["content"].as_str().is_some_and(|s| !s.is_empty()))
    .then_some(Value::Object(data))
}
pub fn full_content(content: &str) -> Option<String> {
    let mut html = Html::parse_document(content);
    let ids = html
        .select(&Selector::parse("script,style,nav,header,footer,aside").unwrap())
        .map(|e| e.id())
        .collect::<Vec<_>>();
    for id in ids {
        if let Some(mut node) = html.tree.get_mut(id) {
            node.detach();
        }
    }
    let mut full = String::new();
    for selector in [
        "article",
        ".content",
        ".article-content",
        ".post-content",
        ".entry-content",
        ".press-release",
        ".news-content",
        "main",
        ".main-content",
        "#content",
        ".body",
    ] {
        let nodes = html
            .select(&Selector::parse(selector).unwrap())
            .collect::<Vec<_>>();
        if !nodes.is_empty() {
            for node in nodes {
                let t = text(node, " ", true);
                if t.chars().count() > full.chars().count() {
                    full = t;
                }
            }
            break;
        }
    }
    if full.is_empty() {
        full = text(html.root_element(), " ", true);
    }
    let full = normalize(&full);
    (full.chars().count() > 100).then_some(full)
}
pub fn rss_ids(xml: &str) -> Result<Vec<i64>, quick_xml::Error> {
    use quick_xml::{events::Event, Reader};
    let mut reader = Reader::from_str(xml);
    let mut item = false;
    let mut ids = Vec::new();
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"item" => item = true,
            Event::End(e) if e.name().as_ref() == b"item" => item = false,
            Event::Start(e) if item && e.name().as_ref() == b"link" => {
                let raw = reader.read_text(e.name())?;
                let raw = raw
                    .strip_prefix("<![CDATA[")
                    .and_then(|s| s.strip_suffix("]]> ").or_else(|| s.strip_suffix("]]>")))
                    .unwrap_or(&raw);
                let raw = quick_xml::escape::unescape(raw).unwrap_or_else(|_| raw.into());
                if let Some(id) = extract_protext_id(&raw).filter(|id| *id != 0) {
                    ids.push(id);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(ids)
}
