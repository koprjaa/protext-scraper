#
# Project: protext-scraper
# File:    parsing.py
#
# Description:
# Turns a Protext.cz article page into its content, keywords, category, and ID.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Turning a Protext.cz page into fields.

Kept apart from the fetching so it can be tested without Tor and without the
site being reachable.
"""

import re

# Article text shorter than this is a stub, a cookie notice or an error page,
# not a press release.
MIN_CONTENT_LENGTH = 50

# Keywords are shorter than this only when the label was stripped and nothing
# real was behind it.
MIN_KEYWORDS_LENGTH = 2

CDATA_RE = re.compile(r"<!\[CDATA\[(.*?)\]\]>", re.S)
HTML_TAG_RE = re.compile(r"<[^>]+>")
WHITESPACE_RE = re.compile(r"\s+")

# Hyphen, en dash and em dash. All three appear as separators around the
# keyword list, so all three come off. The characters are the point here.
DASH_EDGES_RE = re.compile(r"^[-–—\s]+|[-–—\s]+$")  # noqa: RUF001

# Three URL shapes for one article. The site moved from a query parameter to a
# path, and the RSS feed hands out a third form that redirects to the canonical
# one:
#   /zprava.php?id=58633      the original
#   /zprava/58633             what the RSS feed links to
#   /zpravy/show/58633        where both of those land
# The path forms are tried first. A query string carrying something like
# campaign_id would otherwise be read as the article id.
ARTICLE_ID_RE = re.compile(r"/zprav(?:a|y/show)/(\d+)\b|[?&]id=(\d+)\b")

# The article page holds the body in one container and both metadata lists in
# another, each behind a bold label:
#
#   <div class="fulltext"> ... the article ... </div>
#   <div class="fulltext-metadata">
#     <p><strong>Klíčová slova</strong><br/>ČR-Korea-víra-právo-lidská</p>
#     <p><strong>Kategorie</strong><br/><span>Náboženství</span><span>Politika</span></p>
#   </div>
#
# Checked against articles 10000, 30000 and 58633: the whole archive renders
# through this template, so the older selectors this file used to try are gone.
ARTICLE_CLASS = "fulltext"
METADATA_CLASS = "fulltext-metadata"
KEYWORD_LABEL = "Klíčová slova"
CATEGORY_LABEL = "Kategorie"


def clean_content(text: str) -> str:
    """Plain text of an article body, or empty when there is not enough of it."""
    if not text:
        return ""

    text = CDATA_RE.sub(r"\1", text)
    text = HTML_TAG_RE.sub("", text)
    text = WHITESPACE_RE.sub(" ", text.strip())

    if len(text) < MIN_CONTENT_LENGTH:
        return ""
    return text


def clean_keywords(text: str) -> str:
    """Normalized keyword list, or empty when nothing meaningful is left."""
    if not text:
        return ""
    text = DASH_EDGES_RE.sub("", WHITESPACE_RE.sub(" ", text.strip()))
    return text if len(text) > MIN_KEYWORDS_LENGTH else ""


def extract_protext_id(url: str) -> int | None:
    """Article number out of a Protext.cz URL, or None when there is none."""
    if not url or "protext.cz" not in url:
        return None
    match = ARTICLE_ID_RE.search(url)
    if not match:
        return None
    return int(match.group(1) or match.group(2))


def _metadata_paragraph(soup, label: str):
    """The paragraph in the metadata box whose bold label matches, or None."""
    box = soup.find("div", class_=METADATA_CLASS)
    if not box:
        return None
    for paragraph in box.find_all("p"):
        strong = paragraph.find("strong")
        if strong and label in strong.get_text():
            return paragraph
    return None


def extract_content(soup) -> str:
    """Article body, or empty when the page does not carry one.

    Reading the body out of its own container rather than off the whole document
    is what keeps the navigation and the footer out of the text.
    """
    body = soup.find("div", class_=ARTICLE_CLASS)
    return clean_content(body.get_text(" ", strip=True)) if body else ""


def extract_keywords(soup) -> str:
    """Keyword list from a parsed article page."""
    paragraph = _metadata_paragraph(soup, KEYWORD_LABEL)
    if not paragraph:
        return ""
    return clean_keywords(paragraph.get_text(" ", strip=True).replace(KEYWORD_LABEL, "", 1))


def extract_category(soup) -> str:
    """Categories of an article, comma separated, or empty when none are named.

    An article carries several. Returning only the first threw away most of the
    classification the site publishes.
    """
    paragraph = _metadata_paragraph(soup, CATEGORY_LABEL)
    if not paragraph:
        return ""
    names = [span.get_text(strip=True) for span in paragraph.find_all("span")]
    return ", ".join(name for name in names if name)
