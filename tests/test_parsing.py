#
# Project: protext-scraper
# File:    test_parsing.py
#
# Description:
# Tests for turning a Protext.cz page into fields.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Tests for turning a Protext.cz page into fields. No Tor, no network."""

import pytest
from bs4 import BeautifulSoup

from protext_scraper.parsing import (
    MIN_CONTENT_LENGTH,
    clean_content,
    clean_keywords,
    extract_category,
    extract_content,
    extract_keywords,
    extract_protext_id,
)


def soup(markup):
    return BeautifulSoup(markup, "html.parser")


def long_enough(text):
    return text + "x" * max(0, MIN_CONTENT_LENGTH - len(text))


# --- clean_content ----------------------------------------------------------


def test_html_tags_are_removed():
    body = long_enough("Firma oznámila nový produkt na trhu")
    assert clean_content(f"<p>{body}</p>") == body


def test_cdata_wrapper_is_removed():
    body = long_enough("Firma oznámila nový produkt na trhu")
    assert clean_content(f"<![CDATA[{body}]]>") == body


def test_whitespace_is_collapsed():
    assert clean_content("  Firma\n\n  oznámila   nový   produkt na českém trhu už dnes ráno  ") == (
        "Firma oznámila nový produkt na českém trhu už dnes ráno"
    )


@pytest.mark.parametrize("text", ["", None, "Krátké", "x" * (MIN_CONTENT_LENGTH - 1)])
def test_content_below_the_minimum_is_dropped(text):
    assert clean_content(text) == ""


def test_content_at_the_minimum_is_kept():
    assert clean_content("x" * MIN_CONTENT_LENGTH) == "x" * MIN_CONTENT_LENGTH


def test_a_page_that_is_only_markup_is_dropped():
    assert clean_content("<div><span></span></div>") == ""


# --- clean_keywords ---------------------------------------------------------


def test_dashes_around_the_keyword_list_are_stripped():
    assert clean_keywords("- ekonomika, finance -") == "ekonomika, finance"


@pytest.mark.parametrize("dash", ["-", "–", "—"])  # noqa: RUF001 - the characters are the point
def test_every_dash_character_is_stripped(dash):
    assert clean_keywords(f"{dash} ekonomika {dash}") == "ekonomika"


@pytest.mark.parametrize("text", ["", None, "  ", "- -", "ab"])
def test_keywords_with_nothing_behind_the_label_are_dropped(text):
    assert clean_keywords(text) == ""


# --- extract_protext_id -----------------------------------------------------


def test_the_id_is_read_from_the_url():
    assert extract_protext_id("https://www.protext.cz/zprava.php?id=53986") == 53986


def test_the_id_is_found_among_other_parameters():
    assert extract_protext_id("https://www.protext.cz/zprava.php?lang=cs&id=1&x=2") == 1


@pytest.mark.parametrize(
    "url",
    ["", None, "https://example.com/zprava.php?id=1", "https://www.protext.cz/index.php"],
)
def test_a_url_without_a_protext_id_gives_none(url):
    assert extract_protext_id(url) is None


# --- the article page template ------------------------------------------------

# What the live pages carry, checked against articles 10000, 30000 and 58633.
ARTICLE = """
<html><body>
  <div class="nav">Přejít k obsahu | Přejít k hlavnímu menu</div>
  <div class="fulltext">Praha 2. srpna 2026 (PROTEXT) - Tělo článku, dost dlouhé na to,
     aby prošlo minimální délkou, kterou clean_content vyžaduje na skutečný obsah.</div>
  <div class="fulltext-metadata">
    <p><strong>Klíčová slova</strong><br/>ČR-Korea-víra-právo</p>
    <p><strong>Kategorie</strong><br/><span>Náboženství</span><br/><span>Politika, veřejná správa</span><br/></p>
  </div>
</body></html>
"""


# --- extract_content ---------------------------------------------------------


def test_the_body_comes_from_its_own_container():
    assert "Tělo článku" in extract_content(soup(ARTICLE))


def test_the_navigation_is_left_out_of_the_body():
    """Reading the whole document swept the menu into every article."""
    assert "Přejít k obsahu" not in extract_content(soup(ARTICLE))


def test_a_page_without_the_container_gives_empty():
    assert extract_content(soup("<html><body><p>Nic</p></body></html>")) == ""


# --- extract_keywords --------------------------------------------------------


def test_the_keywords_are_read_from_the_metadata_box():
    assert extract_keywords(soup(ARTICLE)) == "ČR-Korea-víra-právo"


def test_the_keyword_label_is_not_part_of_the_result():
    assert "Klíčová slova" not in extract_keywords(soup(ARTICLE))


def test_a_page_with_no_keywords_gives_empty():
    assert extract_keywords(soup("<html><body><p>Text</p></body></html>")) == ""


def test_a_metadata_box_without_a_keyword_paragraph_gives_empty():
    page = soup(
        '<html><body><div class="fulltext-metadata">'
        "<p><strong>Kategorie</strong><br/><span>Sport</span></p></div></body></html>"
    )
    assert extract_keywords(page) == ""


# --- extract_category --------------------------------------------------------


def test_every_category_is_returned_not_only_the_first():
    """An article is filed under several. Returning one threw the rest away."""
    assert extract_category(soup(ARTICLE)) == "Náboženství, Politika, veřejná správa"


def test_the_category_label_is_not_part_of_the_result():
    assert "Kategorie" not in extract_category(soup(ARTICLE))


def test_a_single_category_comes_back_on_its_own():
    page = soup(
        '<html><body><div class="fulltext-metadata">'
        "<p><strong>Kategorie</strong><br/><span>Sport</span></p></div></body></html>"
    )
    assert extract_category(page) == "Sport"


def test_a_page_with_no_category_gives_empty():
    assert extract_category(soup("<html><body></body></html>")) == ""
