#!/usr/bin/env python3
"""
PR/Press Release Content Scraper
Extracts full content from PR and press release RSS feeds and saves them to
a single text file.
"""

import requests
import xml.etree.ElementTree as ET
from datetime import datetime
import re
import chardet
import os
import glob
import time
import random
import json
import threading
from concurrent.futures import ThreadPoolExecutor, as_completed
from bs4 import BeautifulSoup
import subprocess
import socket

# Extended User-Agent rotation list with more variety
USER_AGENTS = [
    # Chrome Windows - latest versions
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/118.0.0.0 Safari/537.36",
    # Chrome macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
    # Firefox Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
    # Firefox macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0",
    # Safari macOS
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 "
    "(KHTML, like Gecko) Version/17.1 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 "
    "(KHTML, like Gecko) Version/16.6 Safari/605.1.15",
    # Edge Windows
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0",
    # Linux Chrome
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/120.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) "
    "Chrome/119.0.0.0 Safari/537.36",
    # Linux Firefox
    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
    "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
    # Mobile Chrome Android
    "Mozilla/5.0 (Linux; Android 14; SM-G998B) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Linux; Android 13; Pixel 7) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/119.0.0.0 Mobile Safari/537.36",
    # Mobile Safari iOS
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) "
    "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (iPad; CPU OS 17_1 like Mac OS X) "
    "AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1",
]


def get_random_user_agent():
    """Get a random User-Agent string."""
    return random.choice(USER_AGENTS)


def check_tor_connection():
    """Check if Tor is running and accessible."""
    try:
        # Test Tor connection
        session = requests.Session()
        session.proxies = {
            "http": "socks5://127.0.0.1:9050",
            "https": "socks5://127.0.0.1:9050",
        }

        # Test with a simple request
        response = session.get("https://httpbin.org/ip", timeout=10)
        if response.status_code == 200:
            ip_info = response.json()
            print(f"Tor connection active - IP: {ip_info.get('origin', 'Unknown')}")
            return True
    except Exception as e:
        print(f"Tor connection failed: {e}")
        return False


def start_tor_service():
    """Start Tor service if not running."""
    try:
        # Check if Tor is already running
        if check_tor_connection():
            return True

        print("Starting Tor service...")

        # Try to start Tor (macOS with Homebrew)
        try:
            subprocess.run(
                ["brew", "services", "start", "tor"], check=True, capture_output=True
            )
            time.sleep(5)  # Wait for Tor to start
        except subprocess.CalledProcessError:
            # Try system Tor
            try:
                subprocess.run(
                    ["sudo", "systemctl", "start", "tor"],
                    check=True,
                    capture_output=True,
                )
                time.sleep(5)
            except subprocess.CalledProcessError:
                print("Could not start Tor service automatically")
                print("Please install and start Tor manually:")
                print("  brew install tor && brew services start tor")
                print("  sudo apt install tor && sudo systemctl start tor")
                return False

        return check_tor_connection()

    except Exception as e:
        print(f"Error starting Tor: {e}")
        return False


def get_tor_session():
    """Create a requests session with Tor proxy."""
    session = requests.Session()
    session.proxies = {
        "http": "socks5://127.0.0.1:9050",
        "https": "socks5://127.0.0.1:9050",
    }
    return session


def renew_tor_circuit():
    """Renew Tor circuit for new IP with better error handling."""
    try:
        # Connect to Tor control port
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(5)  # Add timeout
        sock.connect(("127.0.0.1", 9051))  # Tor control port
        
        # Send authentication
        sock.send(b'AUTHENTICATE ""\r\n')
        response = sock.recv(1024)
        if b"250" not in response:
            print("Tor authentication failed")
            sock.close()
            return False
        
        # Send NEWNYM signal
        sock.send(b"SIGNAL NEWNYM\r\n")
        response = sock.recv(1024)
        if b"250" not in response:
            print("Tor NEWNYM signal failed")
            sock.close()
            return False
            
        sock.close()
        print("Tor circuit renewed - new IP")
        time.sleep(3)  # Wait for circuit to establish
        return True
        
    except socket.timeout:
        print("Tor circuit renewal timeout - continuing without renewal")
        return False
    except ConnectionRefusedError:
        print("Tor control port not accessible - continuing without renewal")
        return False
    except Exception as e:
        print(f"Tor circuit renewal failed: {e} - continuing without renewal")
        return False


def make_request_with_retry(url, max_retries=3, base_delay=1, use_tor=True):
    """Make HTTP request with Tor and advanced anti-blocking techniques."""
    for attempt in range(max_retries):
        try:
            # Minimal delay before request
            time.sleep(random.uniform(0.05, 0.2))  # Reduced from 0.1-0.5

            # Advanced headers to mimic real browser
            headers = {
                "User-Agent": get_random_user_agent(),
                "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,"
                "image/avif,image/webp,image/apng,*/*;q=0.8",
                "Accept-Language": "en-US,en;q=0.7",  # Changed to en-US
                "Accept-Encoding": "gzip, deflate, br",
                "Connection": "keep-alive",
                "Upgrade-Insecure-Requests": "1",
                "Sec-Fetch-Dest": "document",
                "Sec-Fetch-Mode": "navigate",
                "Sec-Fetch-Site": "none",
                "Sec-Fetch-User": "?1",
                "Cache-Control": "max-age=0",
                "DNT": "1",
                "Sec-Ch-Ua": '"Not_A Brand";v="8", "Chromium";v="120", '
                '"Google Chrome";v="120"',
                "Sec-Ch-Ua-Mobile": "?0",
                "Sec-Ch-Ua-Platform": '"Windows"',
            }

            # Create session with Tor proxy if enabled
            if use_tor:
                session = get_tor_session()
            else:
                session = requests.Session()

            session.headers.update(headers)

            # Balanced timeout for stability
            timeout = random.uniform(12, 20)  # Increased from 8-15

            response = session.get(url, timeout=timeout, allow_redirects=True)

            # Handle different response codes - only renew Tor circuit when blocked
            if response.status_code == 429:
                retry_after = int(response.headers.get("Retry-After", 180))
                print(f"Rate limited (429). Waiting {retry_after} seconds...")
                if use_tor:
                    print("Renewing Tor circuit due to rate limit...")
                    renew_tor_circuit()  # Get new IP only when blocked
                time.sleep(retry_after)
                continue
            elif response.status_code == 403:
                print("Forbidden (403). Waiting longer...")
                if use_tor:
                    print("Renewing Tor circuit due to 403...")
                    renew_tor_circuit()  # Get new IP only when blocked
                time.sleep(random.uniform(10, 20))
                continue
            elif response.status_code == 503:
                print("Service unavailable (503). Waiting...")
                if use_tor and attempt >= 1:  # Only renew after first retry
                    print("Renewing Tor circuit due to persistent 503...")
                    renew_tor_circuit()  # Get new IP only when blocked
                time.sleep(random.uniform(10, 20))  # Shorter wait
                continue

            response.raise_for_status()
            return response

        except requests.exceptions.RequestException as e:
            if attempt < max_retries - 1:
                # Progressive backoff with randomization
                delay = base_delay * (2**attempt) + random.uniform(2, 8)
                print(
                    f"Request failed (attempt {attempt + 1}/{max_retries}): "
                    f"{str(e)[:80]}..."
                )
                print(f"Retrying in {delay:.1f} seconds...")

                # Only renew Tor circuit on persistent failures (not on first retry)
                if use_tor and attempt >= 2:  # Only after 2+ failures
                    print("Renewing Tor circuit due to persistent failures...")
                    renew_tor_circuit()

                time.sleep(delay)
            else:
                print(f"Final attempt failed: {str(e)[:80]}...")
                return None

    return None


# Thread-safe file writing and duplicate tracking
FILE_LOCK = threading.Lock()
PROCESSED_IDS = set()  # Global set to track processed IDs


def remove_duplicates_from_json(file_path):
    """Remove duplicate articles from JSON file based on ID."""
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            articles = json.load(f)
        
        # Create dictionary with ID as key to automatically remove duplicates
        unique_articles = {}
        duplicates_count = 0
        
        for article in articles:
            article_id = article.get("id")
            if article_id:
                if article_id in unique_articles:
                    duplicates_count += 1
                else:
                    unique_articles[article_id] = article
            else:
                # Articles without ID - keep them but add a warning
                unique_articles[f"no_id_{len(unique_articles)}"] = article
        
        # Convert back to list
        cleaned_articles = list(unique_articles.values())
        
        # Save cleaned data
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(cleaned_articles, f, ensure_ascii=False, indent=2)
        
        print(f"Removed {duplicates_count} duplicate articles from {file_path}")
        print(f"Original: {len(articles)} articles, Cleaned: {len(cleaned_articles)} articles")
        
        return cleaned_articles
        
    except Exception as e:
        print(f"Error removing duplicates from {file_path}: {e}")
        return None


def save_articles_progressively(articles, output_dir, filename):
    """Save articles to JSON file progressively with thread safety and duplicate prevention."""
    if not articles:
        return

    try:
        with FILE_LOCK:
            file_path = os.path.join(output_dir, filename)

            # Load existing data if file exists
            existing_data = []
            if os.path.exists(file_path):
                try:
                    with open(file_path, "r", encoding="utf-8") as f:
                        existing_data = json.load(f)
                except (json.JSONDecodeError, FileNotFoundError):
                    existing_data = []

            # Create set of existing IDs for fast lookup
            existing_ids = {article.get("id") for article in existing_data if article.get("id")}
            
            # Filter out duplicates from new articles
            new_articles = []
            duplicates_count = 0
            for article in articles:
                article_id = article.get("id")
                if article_id and article_id not in existing_ids:
                    new_articles.append(article)
                    existing_ids.add(article_id)  # Add to set to prevent duplicates within batch
                else:
                    duplicates_count += 1

            # Add only new articles
            existing_data.extend(new_articles)

            # Save updated data
            with open(file_path, "w", encoding="utf-8") as f:
                json.dump(existing_data, f, ensure_ascii=False, indent=2)

            print(
                f"Saved {len(new_articles)} new articles to {filename} "
                f"(Skipped {duplicates_count} duplicates, Total: {len(existing_data)})"
            )
    except IOError as e:
        print(f"Error saving file: {e}")


def clean_content(text):
    """Clean and filter content text."""
    if not text:
        return ""

    # Remove CDATA tags if present
    text = re.sub(r"<!\[CDATA\[(.*?)\]\]>", r"\1", text)

    # Remove HTML tags
    text = re.sub(r"<[^>]+>", "", text)

    # Remove extra whitespace and normalize
    text = re.sub(r"\s+", " ", text.strip())

    # Filter out very short content
    if len(text) < 50:
        return ""

    return text


def fetch_article_by_id(article_id):
    """Fetch article content by ID from Protext.cz."""
    url = f"https://www.protext.cz/zprava.php?id={article_id}"
    try:
        response = make_request_with_retry(url)
        if not response:
            return None

        # Detect encoding
        raw_content = response.content
        detected = chardet.detect(raw_content)
        encoding = detected["encoding"] if detected["encoding"] else "utf-8"

        try:
            content = raw_content.decode(encoding)
        except UnicodeDecodeError:
            content = raw_content.decode("utf-8", errors="ignore")

        # Parse HTML with BeautifulSoup
        soup = BeautifulSoup(content, "html.parser")

        # Extract article data
        article_data = {}

        # Extract title - specific for Protext.cz structure
        title_elem = (
            soup.find("h1", {"itemprop": "name headline"})
            or soup.find("h1")
            or soup.find("title")
        )
        if title_elem:
            article_data["title"] = clean_content(title_elem.get_text())

        # Extract content - specific selectors for Protext.cz
        content_selectors = [
            "#articlebody",  # Main content area
            '[itemprop="articleBody"]',  # Schema.org markup
            ".omega.seven.columns",  # Content column
            'article[role="main"]',  # Main article
            ".article-content",
            ".content",
            "article",
            "#content",
        ]

        full_text = ""
        for selector in content_selectors:
            elements = soup.select(selector)
            if elements:
                for element in elements:
                    # Remove unwanted elements
                    for unwanted in element.select(
                        "script, style, nav, header, footer, aside, .note"
                    ):
                        unwanted.decompose()

                    text = element.get_text(separator=" ", strip=True)
                    if len(text) > len(full_text):
                        full_text = text
                break

        if not full_text:
            # Fallback - get all text but clean it
            for unwanted in soup.select("script, style, nav, header, footer, aside"):
                unwanted.decompose()
            full_text = soup.get_text(separator=" ", strip=True)

        article_data["content"] = clean_content(full_text)
        article_data["link"] = url
        article_data["id"] = article_id

        # Extract date - specific for Protext.cz structure
        date_elem = (
            soup.find("p", {"itemprop": "datePublished"})
            or soup.find("time")
            or soup.find(class_="date")
        )
        if date_elem:
            article_data["date"] = date_elem.get_text().strip()

        # Extract keywords if available - improved search
        keywords_text = ""
        
        # Method 1: Look for paragraph containing "Klíčová slova"
        keywords_elem = soup.find(
            "p", string=lambda text: text and "Klíčová slova" in text
        )
        if keywords_elem:
            keywords_text = (
                keywords_elem.get_text().replace("Klíčová slova", "").strip()
            )
        
        # Method 2: Look for paragraph with strong tag containing "Klíčová slova"
        if not keywords_text:
            keywords_elem = soup.find("p", string=lambda text: text and "Klíčová slova" in text)
            if keywords_elem:
                # Get the full paragraph text
                full_text = keywords_elem.get_text()
                # Remove "Klíčová slova" and clean up
                keywords_text = full_text.replace("Klíčová slova", "").strip()
        
        # Method 3: Look for any element containing "Klíčová slova" text
        if not keywords_text:
            keywords_elem = soup.find(string=lambda text: text and "Klíčová slova" in text)
            if keywords_elem:
                # Get parent element and extract text
                parent = keywords_elem.parent
                if parent:
                    full_text = parent.get_text()
                    keywords_text = full_text.replace("Klíčová slova", "").strip()
        
        # Method 4: Look for alternative keywords labels
        if not keywords_text:
            for keyword_label in ["Keywords", "Klíčová slova", "Tagy", "Tags"]:
                keywords_elem = soup.find(string=lambda text: text and keyword_label in text)
                if keywords_elem:
                    parent = keywords_elem.parent
                    if parent:
                        full_text = parent.get_text()
                        keywords_text = full_text.replace(keyword_label, "").strip()
                        break
        
        # Method 5: Look for meta keywords
        if not keywords_text:
            meta_keywords = soup.find("meta", {"name": "keywords"})
            if meta_keywords and meta_keywords.get("content"):
                keywords_text = meta_keywords.get("content").strip()
        
        # Clean and format keywords
        if keywords_text:
            # Remove extra whitespace and normalize
            keywords_text = re.sub(r'\s+', ' ', keywords_text.strip())
            # Remove leading/trailing dashes and clean up
            keywords_text = re.sub(r'^[-–—\s]+|[-–—\s]+$', '', keywords_text)
            # Only add if we have meaningful content
            if len(keywords_text) > 2:
                article_data["keywords"] = keywords_text

        # Extract category if available
        category_elem = soup.find("span", {"itemprop": "about"})
        if category_elem:
            article_data["category"] = category_elem.get_text().strip()

        return (
            article_data
            if article_data.get("title") and article_data.get("content")
            else None
        )

    except Exception as e:
        print(f"Error fetching article {article_id}: {e}")
        return None


def process_article_id(
    article_id, output_dir=None, filename=None, selected_categories=None
):
    """Process single article ID (for parallel execution) with duplicate prevention."""
    # Check if already processed
    with FILE_LOCK:
        if article_id in PROCESSED_IDS:
            print(f"✗ ID {article_id}: Already processed (duplicate)")
            return None
        PROCESSED_IDS.add(article_id)
    
    article_data = fetch_article_by_id(article_id)
    if article_data:
        # Filter by category if specified
        if selected_categories:
            article_category = article_data.get("category", "Uncategorized")
            if article_category not in selected_categories:
                print(f"✗ ID {article_id}: Category '{article_category}' not selected")
                return None

        # Debug info for keywords
        keywords_info = ""
        if article_data.get("keywords"):
            keywords_info = f" [Keywords: {article_data['keywords'][:30]}...]"
        
        print(f"✓ ID {article_id}: {article_data['title'][:50]}...{keywords_info}")
        return article_data
    else:
        print(f"✗ ID {article_id}: Not found")
        return None


def scan_id_range_parallel_batch(
    min_id,
    max_id,
    step=1,
    max_workers=10,
    batch_size=500,
    output_dir=None,
    filename=None,
    reverse=True,
    save_frequency=50,
    selected_categories=None,
):
    """Scan a large range of IDs using batch processing for efficiency."""
    direction = "NEWEST → OLDEST" if reverse else "OLDEST → NEWEST"
    print(
        f"\nBatch parallel scanning ID range: {min_id} - {max_id} "
        f"(step: {step}, workers: {max_workers}, batch: {batch_size})"
    )
    print(f"Direction: {direction}")

    total_range = max_id - min_id + 1
    total_batches = (total_range + batch_size - 1) // batch_size

    print(
        f"Total range: {total_range} IDs, Processing in {total_batches} "
        f"batches of {batch_size}"
    )
    print(f"Saving every {save_frequency} articles")

    all_found_articles = []
    article_count = 0
    last_save_count = 0

    # Initialize file if needed
    if output_dir and filename:
        with open(os.path.join(output_dir, filename), "w", encoding="utf-8") as f:
            f.write("")  # Create empty file

    # Process in batches (reverse order if requested)
    for batch_num in range(total_batches):
        if reverse:
            # Start from highest ID and go down
            batch_start = max_id - (batch_num * batch_size)
            batch_end = max(batch_start - batch_size + 1, min_id)
            batch_id_list = list(range(batch_start, batch_end - 1, -step))
        else:
            # Start from lowest ID and go up
            batch_start = min_id + (batch_num * batch_size)
            batch_end = min(batch_start + batch_size - 1, max_id)
            batch_id_list = list(range(batch_start, batch_end + 1, step))

        print(
            f"\nProcessing batch {batch_num + 1}/{total_batches}: "
            f"IDs {batch_id_list[0]}-{batch_id_list[-1]} ({direction})"
        )

        batch_found_articles = []

        # Process batch in parallel
        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            # Submit all tasks for this batch
            future_to_id = {
                executor.submit(
                    process_article_id,
                    article_id,
                    output_dir,
                    filename,
                    selected_categories,
                ): article_id
                for article_id in batch_id_list
            }

            # Process completed tasks
            for future in as_completed(future_to_id):
                article_id = future_to_id[future]
                try:
                    article_data = future.result()
                    if article_data:
                        batch_found_articles.append(article_data)
                        article_count += 1

                except Exception as e:
                    print(f"Error processing ID {article_id}: {e}")

        all_found_articles.extend(batch_found_articles)
        print(
            f"Batch {batch_num + 1} complete: Found {len(batch_found_articles)} "
            f"articles (Total: {len(all_found_articles)})"
        )

        # Progressive saving - save every N articles
        current_count = len(all_found_articles)
        if (
            current_count >= save_frequency
            and (current_count - last_save_count) >= save_frequency
        ):
            print(f"Saving {current_count} articles to disk...")
            save_articles_progressively(all_found_articles, output_dir, filename)
            last_save_count = current_count

        # Shorter delay between batches since we have Tor
        if batch_num < total_batches - 1:
            delay = random.uniform(2, 5)  # Increased for stability
            print(f"Waiting {delay:.1f} seconds before next batch...")

            # Optionally renew Tor circuit every few batches
            if batch_num % 15 == 0 and batch_num > 0:  # Every 15 batches (less frequent)
                print("Renewing Tor circuit for fresh IP...")
                renew_tor_circuit()

            time.sleep(delay)

    # Final save of all remaining articles
    if all_found_articles:
        print(f"Final save: {len(all_found_articles)} articles")
        save_articles_progressively(all_found_articles, output_dir, filename)

    print(
        f"\nBatch parallel scan complete: Found {len(all_found_articles)} "
        f"articles in range {min_id}-{max_id}"
    )
    return all_found_articles


def scan_id_range_parallel(
    min_id,
    max_id,
    step=1,
    max_workers=20,
    output_dir=None,
    filename=None,
    selected_categories=None,
):
    """Scan a range of IDs using parallel workers for much faster processing."""
    print(
        f"\nParallel scanning ID range: {min_id} - {max_id} "
        f"(step: {step}, workers: {max_workers})"
    )

    # Create list of IDs to process
    id_list = list(range(min_id, max_id + 1, step))
    found_articles = []
    article_count = 0

    # Initialize file if needed
    if output_dir and filename:
        with open(os.path.join(output_dir, filename), "w", encoding="utf-8") as f:
            f.write("")  # Create empty file

    # Process IDs in parallel
    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        # Submit all tasks
        future_to_id = {
            executor.submit(
                process_article_id,
                article_id,
                output_dir,
                filename,
                selected_categories,
            ): article_id
            for article_id in id_list
        }

        # Process completed tasks
        for future in as_completed(future_to_id):
            article_id = future_to_id[future]
            try:
                article_data = future.result()
                if article_data:
                    found_articles.append(article_data)
                    article_count += 1

                    # Note: Articles are saved progressively in batch mode

                # Progress update
                if len(found_articles) % 100 == 0:
                    print(f"Progress: {len(found_articles)} articles found so far...")

            except Exception as e:
                print(f"Error processing ID {article_id}: {e}")

    print(
        f"\nParallel scan complete: Found {len(found_articles)} "
        f"articles in range {min_id}-{max_id}"
    )
    return found_articles


def scan_id_range(min_id, max_id, step=1, output_dir=None, filename=None):
    """Scan a range of IDs to find all available articles with progressive saving."""
    print(f"\nScanning ID range: {min_id} - {max_id} (step: {step})")
    found_articles = []

    for article_id in range(min_id, max_id + 1, step):
        print(f"Checking ID {article_id}...", end=" ")

        article_data = fetch_article_by_id(article_id)
        if article_data:
            found_articles.append(article_data)
            print(f"✓ Found: {article_data['title'][:50]}...")

            # Save progressively every 5 articles
            if output_dir and filename and len(found_articles) % 5 == 0:
                save_articles_progressively(found_articles, output_dir, filename)
                print(f"Saved {len(found_articles)} articles to disk")
        else:
            print("✗ Not found")

        # Reduced delay for faster scraping
        time.sleep(random.uniform(0.2, 0.8))

        # Progress update every 20 articles
        if article_id % 20 == 0:
            print(f"Progress: {article_id}/{max_id} (found: {len(found_articles)})")

    # Final save
    if output_dir and filename and found_articles:
        save_articles_progressively(found_articles, output_dir, filename)

    print(
        f"\nScan complete: Found {len(found_articles)} articles in range "
        f"{min_id}-{max_id}"
    )
    return found_articles


def extract_protext_id(url):
    """Extract ID number from Protext.cz URL."""
    if not url or "protext.cz" not in url:
        return None

    # Pattern: https://www.protext.cz/zprava.php?id=53986
    match = re.search(r"id=(\d+)", url)
    if match:
        return int(match.group(1))
    return None


def fetch_full_content(url):
    """Fetch full content from article URL."""
    try:
        headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
            "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36"
        }
        response = requests.get(url, headers=headers, timeout=15)
        response.raise_for_status()

        # Detect encoding
        raw_content = response.content
        detected = chardet.detect(raw_content)
        encoding = detected["encoding"] if detected["encoding"] else "utf-8"

        try:
            content = raw_content.decode(encoding)
        except UnicodeDecodeError:
            content = raw_content.decode("utf-8", errors="ignore")

        # Parse HTML with BeautifulSoup
        soup = BeautifulSoup(content, "html.parser")

        # Remove script and style elements
        for script in soup(["script", "style", "nav", "header", "footer", "aside"]):
            script.decompose()

        # Try to find main content area
        content_selectors = [
            "article",
            ".content",
            ".article-content",
            ".post-content",
            ".entry-content",
            ".press-release",
            ".news-content",
            "main",
            ".main-content",
            "#content",
            ".body",
        ]

        full_text = ""
        for selector in content_selectors:
            elements = soup.select(selector)
            if elements:
                for element in elements:
                    text = element.get_text(separator=" ", strip=True)
                    if len(text) > len(full_text):
                        full_text = text
                break

        # If no specific content found, get all text
        if not full_text:
            full_text = soup.get_text(separator=" ", strip=True)

        # Clean up the text
        full_text = re.sub(r"\s+", " ", full_text.strip())

        return full_text if len(full_text) > 100 else None

    except Exception as e:
        print(f"Error fetching full content from {url}: {e}")
        return None


def fetch_latest_rss_articles():
    """Fetch latest articles from RSS feeds to find the newest ID."""
    print("Fetching latest articles from RSS feeds to find newest ID...")

    # Use only main RSS feed for speed
    main_feed = "https://www.protext.cz/rss/cz.php"
    all_ids = []

    try:
        print(f"Checking {main_feed}...")
        response = make_request_with_retry(
            main_feed, use_tor=True, max_retries=2, base_delay=0.5
        )
        if response and response.status_code == 200:
            # Parse RSS content
            root = ET.fromstring(response.text)
            items = root.findall(".//item")

            for item in items:
                link_elem = item.find("link")
                if link_elem is not None and link_elem.text:
                    article_id = extract_protext_id(link_elem.text)
                    if article_id:
                        all_ids.append(article_id)

    except Exception as e:
        print(f"Error fetching {main_feed}: {e}")

    if all_ids:
        max_id = max(all_ids)
        min_id = min(all_ids)
        print(f"Found {len(all_ids)} articles in RSS feed")
        print(f"ID range: {min_id} - {max_id}")
        print(f"Newest article ID: {max_id}")
        return max_id, min_id
    else:
        print("No articles found in RSS feed, using fallback")
        return None, None


def analyze_categories_from_json(json_file_path):
    """Analyze categories from scraped JSON data and return category statistics."""
    try:
        with open(json_file_path, "r", encoding="utf-8") as f:
            articles = json.load(f)

        categories = {}
        total_articles = len(articles)

        for article in articles:
            category = article.get("category", "Uncategorized")
            if category in categories:
                categories[category] += 1
            else:
                categories[category] = 1

        # Sort categories by count (descending)
        sorted_categories = sorted(categories.items(), key=lambda x: x[1], reverse=True)

        print("\nCATEGORY ANALYSIS")
        print(f"Total articles: {total_articles}")
        print(f"Number of categories: {len(categories)}")
        print("\nCategories (article count):")

        for category, count in sorted_categories:
            percentage = (count / total_articles) * 100
            print(f"  {category}: {count} ({percentage:.1f}%)")

        return categories, sorted_categories

    except Exception as e:
        print(f"Error analyzing categories: {e}")
        return {}, []


def save_categories_to_json(categories, output_dir):
    """Save categories analysis to JSON file."""
    try:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        categories_file = os.path.join(output_dir, f"categories_{timestamp}.json")

        categories_data = {
            "analysis_date": datetime.now().isoformat(),
            "total_categories": len(categories),
            "categories": categories,
        }

        with open(categories_file, "w", encoding="utf-8") as f:
            json.dump(categories_data, f, ensure_ascii=False, indent=2)

        print(f"Categories saved to: {categories_file}")
        return categories_file

    except Exception as e:
        print(f"Error saving categories: {e}")
        return None


def filter_articles_by_categories(articles, selected_categories):
    """Filter articles by selected categories."""
    if not selected_categories:
        return articles

    filtered_articles = []
    for article in articles:
        article_category = article.get("category", "Uncategorized")
        if article_category in selected_categories:
            filtered_articles.append(article)

    return filtered_articles


def get_categories_from_file():
    """Try to load categories from existing JSON file."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    categories_file = os.path.join(script_dir, "data", "categories.json")

    if os.path.exists(categories_file):
        try:
            with open(categories_file, "r", encoding="utf-8") as f:
                categories_list = json.load(f)

            # Convert list to sorted tuples format (count = 0 since we don't have stats)
            sorted_categories = [(cat, 0) for cat in categories_list]

            print(f"\nCategories loaded from file: {categories_file}")
            print(f"Available categories ({len(categories_list)}):")
            for i, (category, count) in enumerate(sorted_categories, 1):
                print(f"{i}. {category}")

            return sorted_categories
        except Exception as e:
            print(f"Error loading categories from file: {e}")

    return None


def get_categories_from_sample(latest_id, sample_size=50):
    """Get available categories by scraping a small sample of articles."""
    print(
        f"\nRetrieving available categories from a sample of {sample_size} articles..."
    )

    # Scrape a small sample to get categories
    sample_min = max(1, latest_id - sample_size + 1)
    sample_articles = scan_id_range_parallel_batch(
        sample_min,
        latest_id,
        max_workers=5,
        batch_size=25,
        output_dir=None,  # Don't save to file
        filename=None,
        reverse=True,
        save_frequency=50,
        selected_categories=None,  # Don't filter for sample
    )

    if not sample_articles:
        print("Failed to retrieve a sample of articles for category analysis.")
        return None

    # Analyze categories from sample
    categories = {}
    for article in sample_articles:
        category = article.get("category", "Uncategorized")
        if category in categories:
            categories[category] += 1
        else:
            categories[category] = 1

    # Sort categories by count
    sorted_categories = sorted(categories.items(), key=lambda x: x[1], reverse=True)

    print(f"\nAvailable categories (from {len(sample_articles)} articles):")
    for i, (category, count) in enumerate(sorted_categories, 1):
        percentage = (count / len(sample_articles)) * 100
        print(f"{i}. {category}: {count} articles ({percentage:.1f}%)")

    return sorted_categories


def select_categories_at_start(sorted_categories):
    """Let user select categories at the beginning of scraping."""
    if not sorted_categories:
        return None

    print("\nCATEGORY SELECTION FOR SCRAPING:")
    print("Select the categories you want to scrape.")
    print("Enter category numbers separated by commas (e.g., 1,3,5)")
    print("Or enter 'all' for all categories")

    try:
        choice = input("Your choice: ").strip().lower()

        if choice == "all":
            return [cat[0] for cat in sorted_categories]

        if choice:
            indices = [int(x.strip()) - 1 for x in choice.split(",")]
            selected_categories = [
                sorted_categories[i][0]
                for i in indices
                if 0 <= i < len(sorted_categories)
            ]

            if selected_categories:
                print(f"\nSelected categories: {', '.join(selected_categories)}")
                return selected_categories
            else:
                print("Invalid category numbers")
                return None
        else:
            print("No categories selected - all will be scraped")
            return None

    except ValueError:
        print("Invalid number format")
        return None


def offer_category_filtering(articles, output_dir):
    """Offer category filtering after scraping is complete."""
    if not articles:
        return

    print("\nDO YOU WANT TO FILTER BY CATEGORIES?")
    print(f"Found {len(articles)} articles. You can filter them by categories.")
    filter_choice = input("Enter 'y' to filter or Enter to continue: ").strip().lower()

    if filter_choice == "y":
        # Analyze categories from current articles
        categories = {}
        for article in articles:
            category = article.get("category", "Uncategorized")
            if category in categories:
                categories[category] += 1
            else:
                categories[category] = 1

        # Sort categories by count
        sorted_categories = sorted(categories.items(), key=lambda x: x[1], reverse=True)

        print("\nAvailable categories:")
        for i, (category, count) in enumerate(sorted_categories, 1):
            percentage = (count / len(articles)) * 100
            print(f"{i}. {category}: {count} articles ({percentage:.1f}%)")

        try:
            selected_indices = input(
                "\nEnter category numbers separated by commas (e.g., 1,3,5): "
            ).strip()
            if selected_indices:
                indices = [int(x.strip()) - 1 for x in selected_indices.split(",")]
                selected_categories = [
                    sorted_categories[i][0]
                    for i in indices
                    if 0 <= i < len(sorted_categories)
                ]

                if selected_categories:
                    print(
                        f"\nFiltering by categories: {', '.join(selected_categories)}"
                    )
                    filtered_articles = filter_articles_by_categories(
                        articles, selected_categories
                    )

                    # Save filtered results
                    filtered_filename = f"filtered_content_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
                    save_articles_progressively(
                        filtered_articles, output_dir, filtered_filename
                    )

                    print(
                        f"Filtered {len(filtered_articles)} articles out of "
                        f"{len(articles)}"
                    )
                    print(f"Filtered results saved to: {filtered_filename}")

                    # Also save categories analysis
                    save_categories_to_json(categories, output_dir)
                else:
                    print("Invalid category numbers")
        except ValueError:
            print("Invalid number format")


def main():
    """Main function to scrape Protext.cz articles directly via ID scanning with Tor."""

    # Load and display ASCII art
    script_dir = os.path.dirname(os.path.abspath(__file__))
    ascii_file = os.path.join(script_dir, "data", "ascii.txt")
    try:
        if os.path.exists(ascii_file):
            with open(ascii_file, "r", encoding="utf-8") as f:
                ascii_art = f.read()
                print(ascii_art)
                print()
    except Exception:
        pass  # If ASCII art fails to load, continue anyway

    print("=" * 50)

    # Check and setup Tor
    print("Checking Tor connection...")
    if not check_tor_connection():
        print("Attempting to start Tor service...")
        if not start_tor_service():
            print("Tor is not available. Please install Tor:")
            print("   macOS: brew install tor && brew services start tor")
            print("   Linux: sudo apt install tor && sudo systemctl start tor")
            print("   Windows: Download Tor Browser")
            return

    print("Tor is ready!")
    print()

    # Get latest article ID from RSS feeds
    latest_id, oldest_id = fetch_latest_rss_articles()
    if not latest_id:
        print("Could not determine latest article ID. Using fallback range.")
        latest_id = 200000
        oldest_id = 1

    print()

    # Get categories and let user select which ones to scrape
    print("CATEGORY SELECTION:")
    print("Do you want to select specific categories for scraping?")
    category_choice = (
        input("Enter 'y' to select categories or Enter for all: ").strip().lower()
    )

    selected_categories = None
    if category_choice == "y":
        # First try to load categories from existing file
        sorted_categories = get_categories_from_file()

        # If no file exists, scrape a small sample
        if not sorted_categories:
            sorted_categories = get_categories_from_sample(latest_id, sample_size=50)

        if sorted_categories:
            selected_categories = select_categories_at_start(sorted_categories)
            if not selected_categories:
                print("All categories will be scraped.")
        else:
            print("Failed to retrieve categories. All will be scraped.")
    else:
        print("All categories will be scraped.")

    print()

    # Create output directory and filename
    script_dir = os.path.dirname(os.path.abspath(__file__))
    output_dir = os.path.join(script_dir, "output")
    os.makedirs(output_dir, exist_ok=True)

    # Clean old reports
    for old_file_path in glob.glob(os.path.join(output_dir, "content_*.json")):
        try:
            os.remove(old_file_path)
        except OSError:
            pass

    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    filename = f"content_{timestamp}.json"

    print(f"Output directory: {output_dir}")
    print(f"Output file: {filename}")
    print()

    # Tor scraping menu with dynamic range
    print("🥷 TOR SCRAPING MODE:")
    print(f"1. TEST - range {latest_id-99}-{latest_id} (quick test)")
    print(f"2. SMALL - range {latest_id-999}-{latest_id} (estimate: ~500-800 articles)")
    print(
        f"3. MEDIUM - range {latest_id-9999}-{latest_id} "
        f"(estimate: ~5000-8000 articles)"
    )
    print(
        f"4. LARGE - range {latest_id-49999}-{latest_id} "
        f"(estimate: ~25000-40000 articles)"
    )
    print(
        f"5. MASSIVE - range {latest_id-99999}-{latest_id} "
        f"(estimate: ~50000-80000 articles)"
    )
    print(
        f"6. MAXIMUM - range {oldest_id}-{latest_id} "
        f"(estimate: ~{latest_id-oldest_id} articles)"
    )
    print("7. CUSTOM - enter custom range")
    print("8. CATEGORY ANALYSIS - scrape 200 articles and analyze categories")
    print()
    print("SCRAPING DIRECTION:")
    print("A. NEWEST → OLDEST (recommended - starts with the newest articles)")
    print("B. OLDEST → NEWEST (starts with the oldest articles)")
    print()
    print("SAVE FREQUENCY:")
    print("1. Every 25 articles (very frequent)")
    print("2. Every 50 articles (frequent)")
    print("3. Every 100 articles (normal)")
    print("4. Every 200 articles (less frequent)")

    try:
        choice = input("\nEnter choice (1/2/3/4/5/6/7/8): ").strip()

        # For category analysis (option 8), skip direction and save frequency questions
        if choice != "8":
            # Ask for direction
            direction_choice = (
                input("Enter direction (A/B) [A = NEWEST→OLDEST]: ").strip().upper()
            )
            reverse = direction_choice != "B"  # Default to NEWEST→OLDEST

            # Ask for save frequency
            save_choice = input(
                "Enter save frequency (1/2/3/4) [3 = every 100]: "
            ).strip()
            save_frequency = {"1": 25, "2": 50, "3": 100, "4": 200}.get(
                save_choice, 100
            )  # Default to 100
        else:
            # Default values for category analysis
            reverse = True
            save_frequency = 50

        all_articles = []
        if choice == "1":
            # Test range
            test_min = max(1, latest_id - 99)
            print(f"TEST MODE: {test_min}-{latest_id} (TOR FAST)")
            all_articles = scan_id_range_parallel_batch(
                test_min,
                latest_id,
                max_workers=10,  # Reduced from 20
                batch_size=50,   # Reduced from 100
                output_dir=output_dir,
                filename=filename,
                reverse=reverse,
                save_frequency=save_frequency,
                selected_categories=selected_categories,
            )
        elif choice == "2":
            # Small range
            small_min = max(1, latest_id - 999)
            print(f"SMALL DATASET: {small_min}-{latest_id} (TOR FAST)")
            all_articles = scan_id_range_parallel_batch(
                small_min,
                latest_id,
                max_workers=15,  # Reduced from 30
                batch_size=100,  # Reduced from 200
                output_dir=output_dir,
                filename=filename,
                reverse=reverse,
                save_frequency=save_frequency,
                selected_categories=selected_categories,
            )
        elif choice == "3":
            # Medium range
            medium_min = max(1, latest_id - 9999)
            print(f"MEDIUM DATASET: {medium_min}-{latest_id} (TOR FAST)")
            confirm = input("Continue? (y/N): ").strip().lower()
            if confirm == "y":
                all_articles = scan_id_range_parallel_batch(
                    medium_min,
                    latest_id,
                    max_workers=20,  # Reduced from 40
                    batch_size=500,  # Reduced from 1000
                    output_dir=output_dir,
                    filename=filename,
                    reverse=reverse,
                    save_frequency=save_frequency,
                    selected_categories=selected_categories,
                )
            else:
                print("Cancelled.")
                return
        elif choice == "4":
            # Large range
            large_min = max(1, latest_id - 49999)
            print(f"LARGE DATASET: {large_min}-{latest_id} (TOR FAST)")
            confirm = input("Continue? (y/N): ").strip().lower()
            if confirm == "y":
                all_articles = scan_id_range_parallel_batch(
                    large_min,
                    latest_id,
                    max_workers=25,  # Reduced from 50
                    batch_size=500,  # Reduced from 1000
                    output_dir=output_dir,
                    filename=filename,
                    reverse=reverse,
                    save_frequency=save_frequency,
                    selected_categories=selected_categories,
                )
            else:
                print("Cancelled.")
                return
        elif choice == "5":
            # Massive range
            massive_min = max(1, latest_id - 99999)
            print(f"MASSIVE DATASET: {massive_min}-{latest_id} (TOR FAST)")
            confirm = input("Continue? (y/N): ").strip().lower()
            if confirm == "y":
                all_articles = scan_id_range_parallel_batch(
                    massive_min,
                    latest_id,
                    max_workers=25,  # Reduced from 50
                    batch_size=500,  # Reduced from 1000
                    output_dir=output_dir,
                    filename=filename,
                    reverse=reverse,
                    save_frequency=save_frequency,
                    selected_categories=selected_categories,
                )
            else:
                print("Cancelled.")
                return
        elif choice == "6":
            # Maximum range
            print(f"MAXIMUM DATASET: {oldest_id}-{latest_id} (TOR FAST)")
            confirm = input("Continue? (y/N): ").strip().lower()
            if confirm == "y":
                all_articles = scan_id_range_parallel_batch(
                    oldest_id,
                    latest_id,
                    max_workers=25,  # Reduced from 50
                    batch_size=500,  # Reduced from 1000
                    output_dir=output_dir,
                    filename=filename,
                    reverse=reverse,
                    save_frequency=save_frequency,
                    selected_categories=selected_categories,
                )
            else:
                print("Cancelled.")
                return
        elif choice == "7":
            # Custom range
            try:
                min_id = int(input("Enter minimum ID: "))
                max_id = int(input("Enter maximum ID: "))
                workers = int(
                    input("Enter number of workers (recommended 50): ") or "50"
                )
                batch_size = int(
                    input("Enter batch size (recommended 1000): ") or "1000"
                )

                print(f"CUSTOM DATASET: {min_id}-{max_id}")
                confirm = input("Continue? (y/N): ").strip().lower()
                if confirm == "y":
                    all_articles = scan_id_range_parallel_batch(
                        min_id,
                        max_id,
                        max_workers=workers,
                        batch_size=batch_size,
                        output_dir=output_dir,
                        filename=filename,
                        selected_categories=selected_categories,
                    )
                else:
                    print("Cancelled.")
                    return
            except ValueError:
                print("Invalid number!")
                return
        elif choice == "8":
            # Category analysis mode - scrape 200 articles and analyze categories
            analysis_min = max(1, latest_id - 199)
            print(f"CATEGORY ANALYSIS: {analysis_min}-{latest_id} (200 articles)")

            # Scrape 200 articles for category analysis
            all_articles = scan_id_range_parallel_batch(
                analysis_min,
                latest_id,
                max_workers=8,
                batch_size=50,
                output_dir=output_dir,
                filename=filename,
                reverse=True,
                save_frequency=50,
                selected_categories=None,  # Don't filter for analysis
            )

            if all_articles:
                # Analyze categories
                categories, sorted_categories = analyze_categories_from_json(
                    os.path.join(output_dir, filename)
                )

                if categories:
                    # Save categories to JSON
                    categories_file = save_categories_to_json(categories, output_dir)

                    # Ask if user wants to filter by categories
                    print("\nDO YOU WANT TO FILTER BY CATEGORIES?")
                    filter_choice = (
                        input("Enter 'y' to filter or Enter to continue: ")
                        .strip()
                        .lower()
                    )

                    if filter_choice == "y":
                        print("\nAvailable categories:")
                        for i, (category, count) in enumerate(sorted_categories, 1):
                            print(f"{i}. {category} ({count} articles)")

                        try:
                            selected_indices = input(
                                "\nEnter category numbers separated by commas (e.g., 1,3,5): "
                            ).strip()
                            if selected_indices:
                                indices = [
                                    int(x.strip()) - 1
                                    for x in selected_indices.split(",")
                                ]
                                selected_categories = [
                                    sorted_categories[i][0]
                                    for i in indices
                                    if 0 <= i < len(sorted_categories)
                                ]

                                if selected_categories:
                                    print(
                                        f"\nFiltering by categories: "
                                        f"{', '.join(selected_categories)}"
                                    )
                                    filtered_filename = f"filtered_content_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
                                    filtered_articles = filter_articles_by_categories(
                                        all_articles, selected_categories
                                    )

                                    # Save filtered results
                                    save_articles_progressively(
                                        filtered_articles, output_dir, filtered_filename
                                    )

                                    print(
                                        f"Filtered {len(filtered_articles)} articles "
                                        f"out of {len(all_articles)}"
                                    )
                                    print(
                                        f"Filtered results saved to: {filtered_filename}"
                                    )
                                else:
                                    print("Invalid category numbers")
                        except ValueError:
                            print("Invalid number format")
            else:
                print("No articles found for category analysis")
        else:
            print("Invalid choice!")
            return

    except KeyboardInterrupt:
        print("\n\nBye!")
        return
    except Exception as e:
        print(f"Error during choice: {e}")
        return

    # Final summary
    if "all_articles" in locals() and all_articles:
        print("\nSCRAPING COMPLETE!")
        print(f"Total articles found: {len(all_articles)}")
        print(f"Saved to: {filename}")
        print(f"Location: {output_dir}")
        try:
            file_size = (
                os.path.getsize(os.path.join(output_dir, filename)) / 1024 / 1024
            )
            print(f"File size: {file_size:.1f} MB")
        except OSError:
            print("File size: Calculating...")

    else:
        print("\nNo articles found.")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n\nBye!")
        exit(0)
