# protext-scraper

Concurrent scraper for the Czech press release archive Protext.cz. It enumerates article IDs over Tor and requests a new circuit when the site blocks it.

![rust](https://img.shields.io/badge/rust-stable-DEA584?style=flat-square&logo=rust&logoColor=black)
![license](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)
[![ci](https://github.com/koprjaa/protext-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/protext-scraper/actions/workflows/ci.yml)

Written for a 4IT550 Competitive Intelligence term paper at Prague University of Economics and Business. Protext.cz holds two decades of press releases, blocks automated access, and often serves legacy Windows-1250 encoding.


## Install

Install a current stable Rust toolchain, then run:

```sh
cargo install --path .
```

Tor must run locally with SOCKS on `127.0.0.1:9050`. Circuit renewal uses the control port `127.0.0.1:9051` with empty-password authentication, as in the original. Configure the Tor daemon accordingly if renewal is needed.

```sh
# macOS
brew install tor && brew services start tor

# Debian and Ubuntu
sudo apt install tor && sudo systemctl start tor
```

On Windows, install and start the Tor Expert Bundle using the ports above. The application tries `brew services start tor` and then `sudo systemctl start tor` when its connectivity checks fail.

## Use

Run from the project directory, or from a writable directory containing your optional `data/ascii.txt` and `data/categories.json` files:

```sh
protext-scraper
# Or, from the source tree:
cargo run --release
```

The original CLI is interactive. It defines no switches, and extra process arguments are ignored. The initial Tor check queries `https://httpbin.org/ip`. The scraper finds the latest ID in `https://www.protext.cz/rss/cz.php`, falling back to 200000.

Choose cached categories or all articles, then choose a mode:

| Mode | Inclusive range | Workers | Batch size |
| --- | --- | --- | --- |
| TEST | latest 100 IDs | 10 | 50 |
| SMALL | latest 1,000 IDs | 15 | 100 |
| MEDIUM | latest 10,000 IDs | 20 | 500 |
| LARGE | latest 50,000 IDs | 25 | 500 |
| MASSIVE | latest 100,000 IDs | 25 | 500 |
| MAXIMUM | 1 through latest | 25 | 500 |
| CUSTOM | entered range | entered, default 50 | entered, default 1,000 |
| CATEGORY ANALYSIS | latest 200 IDs | 8 | 50 |

Preset lower bounds are clamped to 1. Medium through maximum and custom modes ask for confirmation. Direction defaults to newest first. Save frequency choices are 25, 50, 100 (default), or 200 articles; saving happens at batch boundaries and once at the end. Analysis mode always scans newest first, saves every 50, and ignores the initial category filter. It produces a category report and optionally a filtered article file.

Output goes to `output/content_YYYYMMDD_HHMMSS.json`. Filtered files use `filtered_content_...json`; category reports use `categories_...json`. JSON is UTF-8, with literal Unicode, two-space indentation, insertion-order fields, and no final newline. Article fields in order are `title`, `content`, `link`, `id`, optional `date`, optional `keywords`, optional `category`. There is no raw HTML field, CSV, XLSX, XML output, database, application-specific environment variable, or additional configuration file. The checked-in category cache is a JSON array of names.

The body comes only from `div.fulltext`; keywords and all categories come from `div.fulltext-metadata`. Multiple category names are joined with comma and space, and the entire joined string is counted or matched as one category. The body must contain at least 50 Unicode characters after cleaning. Titles must be nonempty. Keyword lists require more than two characters. Output deduplication keeps the first non-null article ID; missing and null IDs are retained individually.

Requests retain all 21 original User-Agents and browser headers. Each article gets up to three attempts, with 50–200 ms jitter and a random 12–20 second request timeout. A 429 waits `Retry-After` seconds (default 180); 403 and 503 wait 10–20 seconds. Circuit renewal occurs on 429/403 and from the second 503 attempt. Other failures back off by `base_delay * 2^attempt + U(2,8)`; renewal applies from the third failure only if another attempt remains. RSS uses two attempts and a 0.5 second base delay. Batches pause 2–5 seconds and periodically renew Tor after zero-based batch 15, 30, etc., when another batch remains. Results are stored in worker completion order.

`Ctrl+C` prints `Bye!` and exits successfully. Normal cancellation, unavailable Tor, invalid menu input, and reported runtime errors also retain exit status zero. Errors are printed to stdout without a logging framework. The library additionally exposes sequential scanning, nonbatch parallel scanning, full-content fetching, duplicate cleanup, category filtering, and parsing utilities.

## Development

```sh
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Tests cover each of the original 23 parser and 22 storage test functions, including their parameterized cases. Additional tests cover every menu mode, cancellation, category reports, encoding, HTTP redirects and headers, Tor control, retries, concurrency, persistence, and the executable. All tests use deterministic transports or local TCP fixtures; none contact a live website. TCP fixtures explicitly restore blocking mode after accept and drain input after shutting down writes for Windows compatibility. CI runs on Ubuntu and Windows.

Dependencies are limited to `reqwest` (blocking HTTP, rustls, SOCKS, compression), `scraper` (HTML), `quick-xml` (RSS), `serde_json` (ordered JSON), `regex` (original cleaning patterns), `rand` (agents and jitter), `chardetng`/`encoding_rs` (legacy decoding), `chrono` (local timestamps), and `ctrlc` (successful interrupt exit). `tempfile` is used only by tests. Bounded standard-library threads replace the executor; no explicit async runtime dependency or CLI parser is needed.

## Changes from the Python version

- Short, nonempty titles are accepted. Python applied the body's 50-character minimum to titles and silently dropped otherwise valid releases. The 50-character body threshold is unchanged.

- The executable uses the current working directory for `data/` and `output/`, replacing Python's source-file directory. This makes `cargo install --path .` usable without a retained source checkout. Missing optional data files remain nonfatal.
- Earlier `content_*.json` reports are preserved instead of deleted at startup. An existing scan output file is no longer truncated. Empty scans create valid `[]` rather than an empty, invalid JSON file. Saves still rewrite the JSON array under a process-local lock; they are not crash-atomic or coordinated across processes.
- Sampling without an output path now skips saving instead of failing. Processed IDs are scoped to each scan instead of persisting across sampling and subsequent scans, allowing retries in a later scan. Within one scan, IDs are still marked before fetching.
- Custom mode now honors the selected direction and save frequency; Python accidentally omitted both arguments.
- Reversed ranges and zero/nonpositive workers, step, batch size, or save frequency are rejected with a reported error instead of triggering division errors, executor errors, or invalid indexing. Parsing a negative worker/batch count reports `Invalid number!`.
- Service startup falls back to systemctl when brew is missing, not only when brew returns a failure status. Missing service tools produce the existing manual-install hints instead of a platform-specific exception.
- Invalid, negative, date-form, or overflowing `Retry-After` values use the 180-second fallback instead of raising an uncaught conversion/sleep error. Integer seconds remain the supported format.
- Encoding detection uses chardetng rather than chardet. Valid UTF-8 is decoded directly; legacy guesses and malformed-byte recovery may differ. Replacement characters are retained for invalid sequences, instead of the original UTF-8-ignore fallback. UTF-8 BOMs are removed.
- HTML uses the HTML5 parser in scraper rather than BeautifulSoup's html.parser. Malformed HTML recovery, implicit elements, and unusual nested metadata may differ. RSS uses quick-xml and unescapes link entities and CDATA. Article IDs use signed 64-bit integers rather than arbitrary-precision Python integers.
- HTTP uses reqwest/rustls rather than requests and the system TLS stack. Timeout is a total request deadline (including body reads), rather than requests' per-operation connect/read timeout. Redirect limits, TLS trust, compression, cookies, proxy-environment precedence, and library error text can differ: redirects retain the 30-hop limit; HTTP errors are formatted as `<status> HTTP error for url: <url>`. There is no persistent cookie jar. Explicit Tor routing takes precedence over environment proxies. Direct HTTP uses reqwest's native environment handling; Python-specific `REQUESTS_CA_BUNDLE`/`CURL_CA_BUNDLE`, `NETRC`, and requests authentication behavior are not implemented. Browser header values are preserved; header capitalization/order and automatically added transport headers can differ. Tor connectivity checks use reqwest's default headers rather than requests' package User-Agent.
- JSON parsing is strict JSON: nonfinite `NaN`/`Infinity` values accepted by Python are rejected, and numbers use serde_json's integer/finite-float representation rather than arbitrary-precision Python integers. Invalid UTF-8 JSON files load as empty instead of raising a Unicode decoding exception. Helpers tolerate nonobject array entries as records without IDs and ignore nonstring category values; Python could raise attribute/type errors. Structured JSON IDs are compared by serialized representation rather than raising Python's unhashable-ID error. Ordinary integer/string IDs retain first-copy semantics, including numeric/boolean equality.
- Exact random sequences and concurrent completion/log order differ across runtimes. Sampling is uniform over the same agents and timing ranges. Category report timestamps always include six fractional digits; Python omits them at exactly zero microseconds. JSON output uses LF consistently, including Windows (Python text writes translated newlines on Windows). Library callers use typed Rust options instead of Python nullable arguments and exceptions; absent input strings map to empty text before cleaning.
- End-of-input exits gracefully with `Bye!` and status zero instead of an EOF traceback or `Error during choice` message. Interrupt exits immediately; Python's executor could wait for active workers. OS and HTTP error details use Rust messages while application message templates remain English and otherwise retain their original wording. Invalid category cache errors use a stable English message.

## License

[MIT](LICENSE).
