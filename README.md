# protext-scraper

Concurrent scraper for the Czech press release archive Protext.cz. It enumerates article IDs over Tor and requests a new circuit whenever the site blocks it.

![python](https://img.shields.io/badge/python-3.13-3776AB?style=flat-square&logo=python&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-prototype-lightgrey?style=flat-square)
[![ci](https://github.com/koprjaa/protext-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/protext-scraper/actions/workflows/ci.yml)

Written for a 4IT550 Competitive Intelligence term paper at Prague University of Economics and Business. The task was to extract data from a source that resists it. Protext.cz holds two decades of press releases, blocks automated access, and often serves legacy `windows-1250` encoding.

## Install

Tor must run locally first.

```bash
# macOS
brew install tor && brew services start tor

# Debian and Ubuntu
sudo apt install tor && sudo systemctl start tor

# Windows: install the Tor Expert Bundle and run tor.exe with the default ports 9050 and 9051.
```

Then install the Python dependencies:

```bash
uv venv
uv pip install -r requirements.txt
```

## Use

```bash
python main.py
```

A menu asks for the scrape mode:

```
TOR SCRAPING MODE:
1. TEST    range 199900-200000 (quick test)
2. LATEST  last 100 IDs from RSS
3. CUSTOM  specify range
```

Output goes to `output/*.json`. Each entry holds the title, date, keywords, body text, and the raw HTML.

## How it works

```
ID range or RSS feed IDs
        |
        v
ThreadPoolExecutor  <-->  Tor SOCKS5 on 127.0.0.1:9050
        |                        ^
        | on 429 or 403          | control port 9051
        +--- renew circuit ------+
```

Five points explain the design.

- **ID enumeration instead of pagination.** Article URLs carry sequential IDs. Walking 199900 to 200000 is more reliable and more complete than a paginated crawl, which caps out or skips entries.
- **Circuit renewal.** IP rotation alone does not work, because the site profiles behavior. On a 429 or a 403 the worker sends `NEWNYM` to the Tor control port for a new exit node, then backs off exponentially.
- **Encoding detection.** Much of the archive predates UTF-8. The scraper runs `chardet` per response and falls back to `errors='replace'`.
- **User-Agent rotation.** Each request picks an agent from a list.
- **Thread safe writes.** A `threading.Lock` guards the JSON append.

Output goes to JSON, not to a database. For one archive the setup cost of a real database is not worth it. JSON is portable, diffable, and easy to grep. A move to Postgres or S3 costs about 20 lines.

## Limits

- Tor is required. Without a running daemon the script prints install hints for the platform and exits.
- The scraper keeps no state between runs beyond the output files. A repeated range requests everything again, then removes duplicates at write time.
- Throughput is bound by the local worker count and Tor latency. Higher volume needs the ID range split across machines.
- An article shorter than 50 characters after the markup comes off is treated as a stub and dropped. A genuinely short release is lost with it.

## Development

```bash
uv run --extra dev ruff check .
uv run --extra dev pytest -q
```

`src/protext_scraper/parsing.py` turns a page into fields and
`src/protext_scraper/storage.py` writes results to disk. Neither imports Tor or
the network, so the suite runs anywhere. CI runs on Python 3.10, 3.11, and 3.12,
across Linux and Windows.

## License

[MIT](LICENSE)
