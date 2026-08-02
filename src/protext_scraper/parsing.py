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

ARTICLE_ID_RE = re.compile(r"id=(\d+)")

# Labels the site has used above the keyword list, in the order they are tried.
KEYWORD_LABELS = ("Klíčová slova", "Keywords", "Tagy", "Tags")


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
    return int(match.group(1)) if match else None


def extract_keywords(soup) -> str:
    """Keyword list from a parsed article page.

    The site has moved this around, so several places are tried in turn: the
    meta tag first because it is unambiguous, then each label that has appeared
    above the list.
    """
    meta = soup.find("meta", {"name": "keywords"})
    if meta and meta.get("content"):
        cleaned = clean_keywords(meta["content"])
        if cleaned:
            return cleaned

    for label in KEYWORD_LABELS:
        # The default argument binds the label now. A bare closure over the loop
        # variable would read whatever the loop had reached by the time it ran.
        node = soup.find(string=lambda text, wanted=label: text and wanted in text)
        if node and node.parent:
            cleaned = clean_keywords(node.parent.get_text().replace(label, ""))
            if cleaned:
                return cleaned
    return ""


def extract_category(soup) -> str:
    """Category of an article, or empty when the page does not name one."""
    node = soup.find("span", {"itemprop": "about"})
    return node.get_text().strip() if node else ""
