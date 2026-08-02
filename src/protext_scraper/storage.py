"""Writing scraped articles to disk.

Workers append to one file, so every write goes through a lock and every
article is checked against what is already there. Rerunning a range therefore
costs requests but does not duplicate rows.
"""

import json
import threading
from pathlib import Path

FILE_LOCK = threading.Lock()


def load_articles(file_path) -> list:
    """Articles already on disk. An unreadable or missing file reads as empty."""
    path = Path(file_path)
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return []
    return data if isinstance(data, list) else []


def write_articles(file_path, articles) -> None:
    Path(file_path).write_text(
        json.dumps(articles, ensure_ascii=False, indent=2), encoding="utf-8"
    )


def deduplicate(articles: list) -> tuple[list, int]:
    """Articles with repeats removed, and how many were dropped.

    An article without an id is kept. There is no way to tell one from another,
    so dropping them would lose data the scraper did collect.
    """
    seen_ids = set()
    kept, dropped = [], 0
    for article in articles:
        article_id = article.get("id")
        if article_id is None:
            kept.append(article)
        elif article_id in seen_ids:
            dropped += 1
        else:
            seen_ids.add(article_id)
            kept.append(article)
    return kept, dropped


def remove_duplicates_from_json(file_path) -> list | None:
    """Rewrite a results file with its duplicates removed."""
    articles = load_articles(file_path)
    if not articles:
        return None

    cleaned, dropped = deduplicate(articles)
    write_articles(file_path, cleaned)
    print(f"Removed {dropped} duplicate articles from {file_path}")
    print(f"Original: {len(articles)} articles, Cleaned: {len(cleaned)} articles")
    return cleaned


def save_articles_progressively(articles, output_dir, filename) -> int:
    """Append articles that are not on disk yet. Returns how many were added.

    An article without an id is written, matching what the cleanup pass keeps.
    Only a repeated id counts as a duplicate.
    """
    if not articles:
        return 0

    with FILE_LOCK:
        file_path = Path(output_dir) / filename
        existing = load_articles(file_path)
        known_ids = {a.get("id") for a in existing if a.get("id") is not None}

        fresh, duplicates = [], 0
        for article in articles:
            article_id = article.get("id")
            if article_id is not None and article_id in known_ids:
                duplicates += 1
                continue
            if article_id is not None:
                known_ids.add(article_id)
            fresh.append(article)

        existing.extend(fresh)
        Path(output_dir).mkdir(parents=True, exist_ok=True)
        write_articles(file_path, existing)

    print(
        f"Saved {len(fresh)} new articles to {filename} "
        f"(Skipped {duplicates} duplicates, Total: {len(existing)})"
    )
    return len(fresh)


def count_categories(articles) -> dict[str, int]:
    """How many articles carry each category."""
    counts: dict[str, int] = {}
    for article in articles:
        category = (article.get("category") or "").strip()
        if category:
            counts[category] = counts.get(category, 0) + 1
    return counts


def filter_articles_by_categories(articles, selected) -> list:
    """Articles whose category is in the selected set. No selection keeps all."""
    if not selected:
        return list(articles)
    wanted = set(selected)
    return [a for a in articles if (a.get("category") or "").strip() in wanted]
