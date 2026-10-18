// Project: protext-scraper
// File: parsing.rs
// Description: Extract fields from Protext article pages.
// Author: Jan Alexandr Kopřiva
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

pub fn is_python_whitespace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}
pub fn trim(text: &str) -> &str {
    text.trim_matches(is_python_whitespace)
}
pub fn normalize(text: &str) -> String {
    text.split(is_python_whitespace)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
    let text = text.trim_matches(|c: char| is_python_whitespace(c) || "-–—".contains(c));
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
    let digits = m.get(1).or_else(|| m.get(2))?.as_str();
    digits.chars().try_fold(0_i64, |n, c| {
        n.checked_mul(10)?.checked_add(decimal_digit(c)? as i64)
    })
}
fn decimal_digit(c: char) -> Option<u32> {
    if c.is_ascii_digit() {
        return c.to_digit(10);
    }
    static DECIMAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{Nd}$").unwrap());
    if !DECIMAL.is_match(&c.to_string()) {
        return None;
    }
    // Decimal Unicode blocks are consecutive groups of ten digits.
    let mut start = c as u32;
    while start > 0 && char::from_u32(start - 1).is_some_and(|p| DECIMAL.is_match(&p.to_string())) {
        start -= 1;
    }
    Some((c as u32 - start) % 10)
}
pub fn parse_integer<T: std::str::FromStr>(text: &str) -> Result<T, T::Err> {
    let text = text.trim();
    let chars: Vec<char> = text.chars().collect();
    let mut ascii = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if let Some(digit) = decimal_digit(c) {
            ascii.push(char::from(b'0' + digit as u8));
        } else if c == '_'
            && i > 0
            && i + 1 < chars.len()
            && decimal_digit(chars[i - 1]).is_some()
            && decimal_digit(chars[i + 1]).is_some()
        {
            continue;
        } else {
            ascii.push(c);
        }
    }
    ascii.parse()
}

pub fn first<'a>(html: &'a Html, selector: &str) -> Option<ElementRef<'a>> {
    html.select(&Selector::parse(selector).ok()?).next()
}
pub fn text(element: ElementRef<'_>, separator: &str, strip: bool) -> String {
    // BeautifulSoup excludes script/style strings from ordinary get_text calls.
    element
        .descendants()
        .filter_map(|node| {
            let value = match node.value() {
                scraper::node::Node::Text(t) => &**t,
                scraper::node::Node::Comment(c) => c.strip_prefix("[CDATA[")?.strip_suffix("]]")?,
                _ => return None,
            };
            if node
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|p| matches!(p.value().name(), "script" | "style" | "template"))
            {
                return None;
            }
            let s: &str = value;
            let s = if strip { trim(s) } else { s };
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
pub fn parse_html(content: &str) -> Html {
    // html.parser exposes CDATA as text; HTML5 parsers otherwise discard it as a comment.
    let content = CDATA.replace_all(content, |c: &regex::Captures<'_>| {
        c[1].replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    });
    Html::parse_document(&content)
}
pub fn parse_article(content: &str, id: i64) -> Option<Value> {
    let html = parse_html(content);
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
        data.insert("date".into(), trim(&text(e, "", false)).into());
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
    let mut html = parse_html(content);
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
pub fn rss_ids(xml: &str) -> Result<Vec<i64>, roxmltree::Error> {
    let document = roxmltree::Document::parse(xml)?;
    let mut ids = Vec::new();
    for item in document
        .root_element()
        .descendants()
        .skip(1)
        .filter(|n| n.has_tag_name("item") && n.tag_name().namespace().is_none())
    {
        if let Some(link) = item
            .children()
            .find(|n| n.has_tag_name("link") && n.tag_name().namespace().is_none())
        {
            if let Some(id) = link
                .first_child()
                .filter(|n| n.is_text())
                .and_then(|n| n.text())
                .and_then(extract_protext_id)
                .filter(|id| *id != 0)
            {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}
