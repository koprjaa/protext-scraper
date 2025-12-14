![Status](https://img.shields.io/badge/status-prototype-lightgrey) ![Python](https://img.shields.io/badge/python-3.13-blue) ![License](https://img.shields.io/badge/license-MIT-green)

# Overview
Protext Scraper is a high-performance, concurrent web scraper designed to extract press releases and PR content from Protext.cz. It utilizes Tor circuit rotation and randomized user agents to bypass rate limiting and ensure reliable data collection.

# Motivation
This project was developed to demonstrate engineering capabilities in handling protected web sources, concurrent network programming, and robust error handling. It serves as a proof-of-concept for reliable data extraction infrastructure rather than a production-ready application.

# What This Project Does
The application systematically iterates through article IDs or processes RSS feeds to locate valid content. For each identified article, it downloads the HTML, detects the encoding (handling legacy Windows-1250 often used in Czech web archives), sanitizes the content by stripping non-text elements, and extracts metadata such as title, date, and keywords. The final output is aggregated into a machine-readable JSON format.

# Architecture
The system follows a concurrent worker pool pattern:
1.  **Input Generation**: An ID range is generated based on RSS feed analysis or user input.
2.  **Worker Pool**: A `ThreadPoolExecutor` spawns worker threads to process IDs in parallel.
3.  **Network Layer**: Each request is routed through a local Tor SOCKS5 proxy.
4.  **Resilience Layer**: The system monitors response codes; 429 (Too Many Requests) or 403 (Forbidden) trigger a Tor circuit renewal (New Identity) and exponential backoff.
5.  **Storage**: Validated data is written to a JSON file using a thread-safe locking mechanism.

# Tech Stack
-   **Language**: Python 3.13
-   **Networking**: `requests`, `pysocks` (SOCKS proxy support)
-   **Parsing**: `BeautifulSoup4` (HTML parsing)
-   **Encoding**: `chardet` (Character set detection)
-   **Concurrency**: `concurrent.futures` (Threading)
-   **Proxy**: Tor Service (external dependency)

# Data Sources
-   **Primary**: https://www.protext.cz/ (Direct article access via ID)
-   **Discovery**: https://www.protext.cz/rss/cz.php (RSS feed for latest ID detection)

# Key Design Decisions
-   **ID-based Iteration**: The target site exposes sequential integer IDs for articles. Iterating through these IDs O(1) proved more reliable and exhaustive than traversing pagination, which can be inconsistent or limited in depth.
-   **Tor & User-Agents**: Standard IP rotation was insufficient due to aggressive blocking. Integrating the Tor control port allows the application to programmatically request a new exit node ("New Identity") immediately upon detection of a block, rather than waiting for timeouts.
-   **JSON Storage**: JSON was selected for the output format to prioritize portability and ease of inspection over the complexity of setting up a relational database for this specific demonstration scope.

# Limitations
-   **Tor Dependency**: The application requires a running Tor service on `localhost:9050` and control port on `9051`. It cannot function without this external service.
-   **Stateless Execution**: The scraper does not maintain a persistent state file of scraped IDs between runs. Restarting a job requires manually specifying the range or relying on the "check for duplicates" logic which incurs network overhead.
-   **Vertical Scaling**: As a single-node application, scraping speed is tied to the local machine's resource limits and the latency of the Tor network.

# How to Run
1.  **Install Tor**:
    -   macOS: `brew install tor && brew services start tor`
    -   Linux: `sudo apt install tor && sudo systemctl start tor`
2.  **Install Dependencies**:
    ```bash
    pip install -r requirements.txt
    ```
3.  **Run Application**:
    ```bash
    python3 main.py
    ```

# Example Usage
Upon launching, the interactive menu provides several modes. To scrape the latest 100 articles:

```text
🥷 TOR SCRAPING MODE:
1. TEST - range 199900-200000 (quick test)
...
Enter choice: 1
```

# Future Improvements
-   **Database Backend**: Migrate from JSON to SQLite or PostgreSQL to support resumable scrapes and complex querying.
-   **Dockerization**: Containerize the application and the Tor service into a `docker-compose` setup for one-command deployment.
-   **Distributed Workers**: Decouple the scraping logic from the scheduler to allow multiple worker nodes to scrape distinct ranges concurrently.

# Author
Jan Alexandr Kopřiva
jan.alexandr.kopriva@gmail.com

# License
MIT
