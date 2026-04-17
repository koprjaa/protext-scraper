# protext-scraper

**Concurrent scraper for Czech press-release archive Protext.cz — ID-based enumeration over Tor, with circuit rotation on every block.**

![python](https://img.shields.io/badge/python-3.13-3776AB?style=flat-square&logo=python&logoColor=white)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
![status](https://img.shields.io/badge/status-prototype-lightgrey?style=flat-square)
![tor](https://img.shields.io/badge/Tor-SOCKS5-7E4798?style=flat-square&logo=torproject&logoColor=white)
![requests](https://img.shields.io/badge/requests-2.x-000?style=flat-square)
![bs4](https://img.shields.io/badge/bs4-4.x-777?style=flat-square)

Built for a **4IT550 Competitive Intelligence** semester paper at VŠE — the task was to demonstrate robust data extraction from protected sources. Protext.cz is an archive of PR releases going back two decades, aggressive about blocking automated access and often served in legacy `windows-1250` encoding. This scraper handles both.

## How it works

```
  ┌────────────────────────┐
  │ ID range / RSS feed IDs│
  └──────────┬─────────────┘
             ▼
  ┌────────────────────────┐      ┌───────────────┐
  │  ThreadPoolExecutor    │◀────▶│ Tor SOCKS5    │
  │  (parallel workers)    │      │ 127.0.0.1:9050│
  └──────────┬─────────────┘      └───────────────┘
             │                          ▲
             │   on 429/403 →           │  control port
             │   renew circuit ─────────┘
             ▼
  ┌────────────────────────┐
  │ chardet → bs4 → JSON   │
  │ (thread-safe write)    │
  └────────────────────────┘
```

Key ingredients:

- **ID-based enumeration** — Protext exposes sequential integer article IDs. Iterating 199900→200000 is both more reliable and more exhaustive than paginated crawling, which caps or skips.
- **Tor circuit renewal** — plain IP rotation isn't enough; the site profiles behavior. On 429/403 the worker hits the Tor control port (`9051`) and requests `NEWNYM` for a fresh exit node, then backs off exponentially.
- **Encoding detection** — a good chunk of the archive predates UTF-8. `chardet` per response, `errors='replace'` fallback on decode.
- **Randomised User-Agent** per request from a rotated list.
- **Thread-safe JSON append** with a `threading.Lock` — simple, portable, good enough for a prototype.

## Running

Tor has to be running locally first:

```bash
# macOS
brew install tor && brew services start tor
# Debian/Ubuntu
sudo apt install tor && sudo systemctl start tor
# Windows
# install the Tor Expert Bundle, run tor.exe with default settings (9050/9051)
```

Then:

```bash
uv venv
uv pip install -r requirements.txt
python main.py
```

An interactive menu prompts for scrape mode:

```
TOR SCRAPING MODE:
1. TEST - range 199900-200000 (quick test)
2. LATEST - last 100 IDs from RSS
3. CUSTOM - specify range
...
```

Output lands in `output/*.json`, one article per entry, with title/date/keywords/body/HTML.

## Why JSON, not SQLite

For a demonstrator of one archive the friction of setting up a real DB wasn't worth it — JSON is portable, diffable, and easy to grep. If the next iteration hits Postgres/S3, migration is ~20 lines.

## Known limits

- **Tor is mandatory** — no running daemon, no scraping. Script detects absence and prints platform-specific install hints before exiting.
- **Stateless between runs** — no persistent "already-scraped IDs" index beyond the output JSON. Re-running a range re-requests everything but dedupes at write time.
- **Single-node** — throughput is bounded by local worker count × Tor network latency. For serious volume, shard the ID range across machines.

## License

[MIT](LICENSE)
