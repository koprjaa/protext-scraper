// Project: protext-scraper
// File: network.rs
// Description: HTTP retries, encoding detection, and Tor control.
// Author: Jan Alexandr Kopřiva
// License: MIT

use crate::parsing::{full_content, parse_article, parse_integer, rss_ids};
use rand::Rng;
use reqwest::{blocking::Client, header::HeaderMap, Proxy};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
#[path = "agents.rs"]
mod agents;
pub use agents::USER_AGENTS;

pub struct Response {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
}
pub trait Transport: Send + Sync {
    fn get(
        &self,
        url: &str,
        headers: &HeaderMap,
        timeout: f64,
        tor: bool,
    ) -> Result<Response, String>;
    fn sleep(&self, seconds: f64) {
        if let Ok(duration) = Duration::try_from_secs_f64(seconds) {
            thread::sleep(duration);
        }
    }
    fn random(&self, low: f64, high: f64) -> f64 {
        rand::thread_rng().gen_range(low..high)
    }
    fn renew(&self) -> bool {
        renew_tor_circuit()
    }
}
pub struct HttpTransport;
impl Transport for HttpTransport {
    fn get(
        &self,
        url: &str,
        headers: &HeaderMap,
        timeout: f64,
        tor: bool,
    ) -> Result<Response, String> {
        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::limited(30))
            .timeout(Duration::try_from_secs_f64(timeout).map_err(|e| e.to_string())?)
            .default_headers(headers.clone())
            .gzip(true)
            .brotli(true)
            .deflate(true);
        if tor {
            builder =
                builder.proxy(Proxy::all("socks5h://127.0.0.1:9050").map_err(|e| e.to_string())?);
        }
        let response = builder
            .build()
            .and_then(|c| c.get(url).send())
            .map_err(|e| e.to_string())?;
        Ok(Response {
            status: response.status().as_u16(),
            headers: response.headers().clone(),
            body: response.bytes().map_err(|e| e.to_string())?.to_vec(),
        })
    }
}
pub fn browser_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name,value) in [
        ("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"),
        ("accept-language", "en-US,en;q=0.7"), ("accept-encoding", "gzip, deflate, br"), ("connection", "keep-alive"),
        ("upgrade-insecure-requests", "1"), ("sec-fetch-dest", "document"), ("sec-fetch-mode", "navigate"),
        ("sec-fetch-site", "none"), ("sec-fetch-user", "?1"), ("cache-control", "max-age=0"), ("dnt", "1"),
        ("sec-ch-ua", "\"Not_A Brand\";v=\"8\", \"Chromium\";v=\"120\", \"Google Chrome\";v=\"120\""),
        ("sec-ch-ua-mobile", "?0"), ("sec-ch-ua-platform", "\"Windows\""),
    ] { headers.insert(name, value.parse().unwrap()); }
    headers.insert(
        "user-agent",
        USER_AGENTS[rand::thread_rng().gen_range(0..USER_AGENTS.len())]
            .parse()
            .unwrap(),
    );
    headers
}
#[derive(Clone)]
pub struct Network {
    pub transport: Arc<dyn Transport>,
    /// IDs whose fetch failed after all retries, as opposed to IDs that do not exist.
    pub failed: Arc<Mutex<Vec<i64>>>,
}
impl Default for Network {
    fn default() -> Self {
        Self {
            transport: Arc::new(HttpTransport),
            failed: Arc::default(),
        }
    }
}
impl Network {
    pub fn request(
        &self,
        url: &str,
        retries: usize,
        base_delay: f64,
        tor: bool,
    ) -> Option<Response> {
        self.attempt(url, retries, base_delay, tor).ok()
    }
    /// `Err(true)` means the page does not exist, `Err(false)` that every attempt failed.
    fn attempt(
        &self,
        url: &str,
        retries: usize,
        base_delay: f64,
        tor: bool,
    ) -> Result<Response, bool> {
        for attempt in 0..retries {
            self.transport.sleep(self.transport.random(0.05, 0.2));
            let result = self.transport.get(
                url,
                &browser_headers(),
                self.transport.random(12.0, 20.0),
                tor,
            );
            let error = match result {
                Ok(r) => match r.status {
                    429 => {
                        let delay = r
                            .headers
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| parse_integer::<u64>(v).ok())
                            .unwrap_or(180)
                            .min(86_400);
                        println!("Rate limited (429). Waiting {delay} seconds...");
                        if tor {
                            println!("Renewing Tor circuit due to rate limit...");
                            self.transport.renew();
                        }
                        self.transport.sleep(delay as f64);
                        continue;
                    }
                    403 => {
                        println!("Forbidden (403). Waiting longer...");
                        if tor {
                            println!("Renewing Tor circuit due to 403...");
                            self.transport.renew();
                        }
                        self.transport.sleep(self.transport.random(10.0, 20.0));
                        continue;
                    }
                    503 => {
                        println!("Service unavailable (503). Waiting...");
                        if tor && attempt >= 1 {
                            println!("Renewing Tor circuit due to persistent 503...");
                            self.transport.renew();
                        }
                        self.transport.sleep(self.transport.random(10.0, 20.0));
                        continue;
                    }
                    404 | 410 => return Err(true),
                    400..=599 => format!("{} HTTP error for url: {url}", r.status),
                    _ => return Ok(r),
                },
                Err(e) => e,
            };
            let error = error.chars().take(80).collect::<String>();
            if attempt + 1 < retries {
                let delay =
                    base_delay * 2_f64.powi(attempt as i32) + self.transport.random(2.0, 8.0);
                println!(
                    "Request failed (attempt {}/{}): {error}...",
                    attempt + 1,
                    retries
                );
                println!("Retrying in {delay:.1} seconds...");
                if tor && attempt >= 2 {
                    println!("Renewing Tor circuit due to persistent failures...");
                    self.transport.renew();
                }
                self.transport.sleep(delay);
            } else {
                println!("Final attempt failed: {error}...");
            }
        }
        Err(false)
    }
    pub fn fetch_article_by_id(&self, id: i64) -> Option<Value> {
        let url = format!("https://www.protext.cz/zpravy/show/{id}");
        let r = match self.attempt(&url, 3, 1.0, true) {
            Ok(r) => r,
            Err(missing) => {
                if !missing {
                    self.failed.lock().unwrap().push(id);
                }
                return None;
            }
        };
        parse_article(&decode(&r.body), id)
    }
    pub fn fetch_full_content(&self, url: &str) -> Option<String> {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36".parse().unwrap());
        match self.transport.get(url, &headers, 15.0, false) {
            Ok(r) if r.status < 400 => full_content(&decode(&r.body)),
            Ok(r) => {
                println!("Error fetching full content from {url}: HTTP {}", r.status);
                None
            }
            Err(e) => {
                println!("Error fetching full content from {url}: {e}");
                None
            }
        }
    }
    pub fn check_tor_connection(&self) -> bool {
        match self
            .transport
            .get("https://httpbin.org/ip", &HeaderMap::new(), 30.0, true)
        {
            Ok(r) if r.status == 200 => match serde_json::from_slice::<Value>(&r.body) {
                Ok(v) => {
                    println!(
                        "Tor connection active - IP: {}",
                        v.get("origin").and_then(Value::as_str).unwrap_or("Unknown")
                    );
                    true
                }
                Err(e) => {
                    println!("Tor connection failed: {e}");
                    false
                }
            },
            Err(e) => {
                println!("Tor connection failed: {e}");
                false
            }
            _ => false,
        }
    }
    pub fn fetch_latest_rss_articles(&self) -> Option<(i64, i64)> {
        println!("Fetching latest articles from RSS feeds to find newest ID...");
        let url = "https://www.protext.cz/rss/cz.php";
        println!("Checking {url}...");
        if let Some(r) = self.request(url, 2, 0.5, true).filter(|r| r.status == 200) {
            match rss_ids(&String::from_utf8_lossy(&r.body)) {
                Ok(ids) if !ids.is_empty() => {
                    let max = *ids.iter().max().unwrap();
                    let min = *ids.iter().min().unwrap();
                    println!("Found {} articles in RSS feed", ids.len());
                    println!("ID range: {min} - {max}\nNewest article ID: {max}");
                    return Some((max, min));
                }
                Err(e) => println!("Error fetching {url}: {e}"),
                _ => {}
            }
        }
        println!("No articles found in RSS feed, using fallback");
        None
    }
}
pub fn decode(bytes: &[u8]) -> String {
    if let Some((encoding, skip)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[skip..]);
        return text.into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.into();
    }
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    let (decoded, _, _) = detector.guess(None, true).decode(bytes);
    decoded.into_owned()
}
pub fn renew_at(address: SocketAddr) -> std::io::Result<bool> {
    let mut sock = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
    sock.set_read_timeout(Some(Duration::from_secs(5)))?;
    sock.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut buffer = [0; 1024];
    for (command, failure) in [
        (
            b"AUTHENTICATE \"\"\r\n".as_slice(),
            "Tor authentication failed",
        ),
        (b"SIGNAL NEWNYM\r\n".as_slice(), "Tor NEWNYM signal failed"),
    ] {
        sock.write_all(command)?;
        let n = sock.read(&mut buffer)?;
        if !buffer[..n].windows(3).any(|s| s == b"250") {
            println!("{failure}");
            return Ok(false);
        }
    }
    Ok(true)
}
pub fn renew_tor_circuit() -> bool {
    match renew_at("127.0.0.1:9051".parse().unwrap()) {
        Ok(true) => {
            println!("Tor circuit renewed - new IP");
            thread::sleep(Duration::from_secs(3));
            true
        }
        Ok(false) => false,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => {
                    println!("Tor circuit renewal timeout - continuing without renewal")
                }
                std::io::ErrorKind::ConnectionRefused => {
                    println!("Tor control port not accessible - continuing without renewal")
                }
                _ => println!("Tor circuit renewal failed: {e} - continuing without renewal"),
            }
            false
        }
    }
}
