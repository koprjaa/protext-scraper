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


# --- extract_keywords -------------------------------------------------------


def test_the_meta_tag_is_preferred():
    page = soup(
        '<html><head><meta name="keywords" content="ekonomika, finance"></head>'
        "<body><p>Klíčová slova: něco jiného</p></body></html>"
    )
    assert extract_keywords(page) == "ekonomika, finance"


@pytest.mark.parametrize("label", ["Klíčová slova", "Keywords", "Tagy", "Tags"])
def test_every_label_the_site_has_used_is_recognized(label):
    page = soup(f"<html><body><p>{label} ekonomika, finance</p></body></html>")
    assert extract_keywords(page) == "ekonomika, finance"


def test_the_label_lookup_reads_the_label_it_is_on():
    """A closure over the loop variable would test against the last label."""
    page = soup("<html><body><p>Tagy energetika</p></body></html>")
    assert extract_keywords(page) == "energetika"


def test_a_page_with_no_keywords_gives_empty():
    assert extract_keywords(soup("<html><body><p>Text</p></body></html>")) == ""


def test_an_empty_meta_tag_falls_through_to_the_labels():
    page = soup(
        '<html><head><meta name="keywords" content=""></head>'
        "<body><p>Klíčová slova ekonomika</p></body></html>"
    )
    assert extract_keywords(page) == "ekonomika"


# --- extract_category -------------------------------------------------------


def test_the_category_is_read_from_the_itemprop_span():
    page = soup('<html><body><span itemprop="about"> Ekonomika </span></body></html>')
    assert extract_category(page) == "Ekonomika"


def test_a_page_with_no_category_gives_empty():
    assert extract_category(soup("<html><body></body></html>")) == ""
