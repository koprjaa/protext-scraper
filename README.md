# protext-scraper

Downloads Protext.cz press releases for competitive intelligence research.

[![CI](https://github.com/koprjaa/protext-scraper/actions/workflows/ci.yml/badge.svg)](https://github.com/koprjaa/protext-scraper/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-A31F34?style=flat-square)](LICENSE)

Written for a 4IT550 Competitive Intelligence term paper at Prague University of Economics and Business.

## Install

Install a stable Rust toolchain and Tor. From the repository directory, build the executable:

```sh
cargo build --release
```

Start Tor with a local SOCKS proxy on port `9050`. Circuit renewal also needs a local control port on `9051`; the scraper uses empty-password authentication. Configure Tor to match these settings. The scraper tries Homebrew services and then systemctl if its connection check fails.

## Use

Run from the repository directory:

```sh
./target/release/protext-scraper
```

On Windows, run `target\release\protext-scraper.exe`. The menu accepts interactive input. It finds the latest article ID through the Protext.cz RSS feed.

For a first scan, press Enter at category selection, choose `1` for the latest 100 IDs, then press Enter for both direction and save frequency. The defaults scan newest first and save at batch boundaries after at least 100 articles accumulate since the previous save.

For a specific ID range, choose `7`, enter direction and save frequency, then the minimum and maximum IDs. Press Enter for 50 workers and a batch size of 1,000, then confirm with `y`.

Results go to `output/content_YYYYMMDD_HHMMSS.json` under the current directory. Records include title, content, link and ID, plus date, keywords and category when available. Choose `8` to analyze categories across the latest 200 IDs and optionally save filtered results.

## License

[MIT](LICENSE)
