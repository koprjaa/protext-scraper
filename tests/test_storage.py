#
# Project: protext-scraper
# File:    test_storage.py
#
# Description:
# Tests for reading and writing the scraped articles and for dropping repeats.
#
# Author:
# Jan Alexandr Kopřiva
# jan.alexandr.kopriva@gmail.com
#
# License: MIT
#

"""Tests for writing scraped articles to disk."""

import json

from protext_scraper.storage import (
    count_categories,
    deduplicate,
    filter_articles_by_categories,
    load_articles,
    remove_duplicates_from_json,
    save_articles_progressively,
)


def article(article_id, **extra):
    return {"id": article_id, "title": f"Zpráva {article_id}", **extra}


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


# --- load_articles ----------------------------------------------------------


def test_a_missing_file_reads_as_empty(tmp_path):
    assert load_articles(tmp_path / "nope.json") == []


def test_a_truncated_file_reads_as_empty(tmp_path):
    """A run killed mid-write leaves broken JSON. That must not stop the next run."""
    path = tmp_path / "out.json"
    path.write_text('[{"id": 1}, {"id":', encoding="utf-8")
    assert load_articles(path) == []


def test_a_file_holding_something_other_than_a_list_reads_as_empty(tmp_path):
    path = tmp_path / "out.json"
    path.write_text('{"id": 1}', encoding="utf-8")
    assert load_articles(path) == []


# --- deduplicate ------------------------------------------------------------


def test_a_repeated_id_is_dropped_once():
    kept, dropped = deduplicate([article(1), article(2), article(1)])
    assert [a["id"] for a in kept] == [1, 2]
    assert dropped == 1


def test_the_first_copy_of_an_id_is_the_one_kept():
    kept, _ = deduplicate([{"id": 1, "title": "první"}, {"id": 1, "title": "druhý"}])
    assert kept[0]["title"] == "první"


def test_an_article_without_an_id_is_kept():
    """There is no way to tell two of them apart, so dropping them loses data."""
    kept, dropped = deduplicate([{"title": "bez id"}, {"title": "taky bez id"}])
    assert len(kept) == 2
    assert dropped == 0


def test_deduplicating_an_empty_list():
    assert deduplicate([]) == ([], 0)


# --- remove_duplicates_from_json --------------------------------------------


def test_the_results_file_is_rewritten_without_duplicates(tmp_path):
    path = tmp_path / "out.json"
    path.write_text(json.dumps([article(1), article(2), article(1)]), encoding="utf-8")
    remove_duplicates_from_json(path)
    assert [a["id"] for a in read(path)] == [1, 2]


def test_cleaning_a_missing_file_gives_none(tmp_path):
    assert remove_duplicates_from_json(tmp_path / "nope.json") is None


# --- save_articles_progressively --------------------------------------------


def test_articles_are_written(tmp_path):
    assert save_articles_progressively([article(1), article(2)], tmp_path, "out.json") == 2
    assert [a["id"] for a in read(tmp_path / "out.json")] == [1, 2]


def test_a_second_batch_appends(tmp_path):
    save_articles_progressively([article(1)], tmp_path, "out.json")
    save_articles_progressively([article(2)], tmp_path, "out.json")
    assert [a["id"] for a in read(tmp_path / "out.json")] == [1, 2]


def test_an_article_already_on_disk_is_not_written_again(tmp_path):
    save_articles_progressively([article(1)], tmp_path, "out.json")
    assert save_articles_progressively([article(1), article(2)], tmp_path, "out.json") == 1
    assert [a["id"] for a in read(tmp_path / "out.json")] == [1, 2]


def test_a_repeat_inside_one_batch_is_written_once(tmp_path):
    save_articles_progressively([article(1), article(1)], tmp_path, "out.json")
    assert [a["id"] for a in read(tmp_path / "out.json")] == [1]


def test_an_article_without_an_id_is_written(tmp_path):
    """The cleanup pass keeps these, so the writer must not silently drop them."""
    save_articles_progressively([{"title": "bez id"}], tmp_path, "out.json")
    assert len(read(tmp_path / "out.json")) == 1


def test_an_empty_batch_writes_nothing(tmp_path):
    assert save_articles_progressively([], tmp_path, "out.json") == 0
    assert not (tmp_path / "out.json").exists()


def test_a_missing_output_directory_is_created(tmp_path):
    target = tmp_path / "nested" / "run"
    save_articles_progressively([article(1)], target, "out.json")
    assert (target / "out.json").exists()


def test_writing_over_a_broken_file_does_not_lose_the_new_batch(tmp_path):
    path = tmp_path / "out.json"
    path.write_text("not json at all", encoding="utf-8")
    save_articles_progressively([article(1)], tmp_path, "out.json")
    assert [a["id"] for a in read(path)] == [1]


# --- categories -------------------------------------------------------------


def test_categories_are_counted():
    articles = [
        article(1, category="Ekonomika"),
        article(2, category="Ekonomika"),
        article(3, category="Sport"),
    ]
    assert count_categories(articles) == {"Ekonomika": 2, "Sport": 1}


def test_articles_without_a_category_are_not_counted():
    assert count_categories([article(1), article(2, category="  ")]) == {}


def test_filtering_keeps_only_the_selected_categories():
    articles = [article(1, category="Ekonomika"), article(2, category="Sport")]
    assert [a["id"] for a in filter_articles_by_categories(articles, ["Sport"])] == [2]


def test_filtering_with_no_selection_keeps_everything():
    articles = [article(1, category="Ekonomika"), article(2)]
    assert len(filter_articles_by_categories(articles, [])) == 2


def test_filtering_matches_a_category_with_stray_whitespace():
    articles = [article(1, category="  Ekonomika  ")]
    assert len(filter_articles_by_categories(articles, ["Ekonomika"])) == 1
