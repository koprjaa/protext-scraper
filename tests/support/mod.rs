// Project: protext-scraper
// File: tests/support/mod.rs
// Description: Deterministic offline transport for workflow tests.
// Author: Jan Alexandr Kopřiva
// License: MIT
#![allow(dead_code)]
use protext_scraper::network::{Network, Response, Transport};
use reqwest::header::HeaderMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
pub struct Mock {
    pub requests: Mutex<Vec<(String, HeaderMap, f64, bool)>>,
    pub sleeps: Mutex<Vec<f64>>,
    pub renewals: AtomicUsize,
    pub statuses: Mutex<std::collections::VecDeque<Result<u16, String>>>,
    pub retry_after: Mutex<Option<String>>,
    pub body: Mutex<Option<Vec<u8>>>,
    pub active: AtomicUsize,
    pub peak: AtomicUsize,
}
impl Default for Mock {
    fn default() -> Self {
        Self {
            requests: Mutex::new(Vec::new()),
            sleeps: Mutex::new(Vec::new()),
            renewals: AtomicUsize::new(0),
            statuses: Mutex::new(Default::default()),
            retry_after: Mutex::new(None),
            body: Mutex::new(None),
            active: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }
}
impl Transport for Mock {
    fn get(
        &self,
        url: &str,
        headers: &HeaderMap,
        timeout: f64,
        tor: bool,
    ) -> Result<Response, String> {
        self.requests
            .lock()
            .unwrap()
            .push((url.into(), headers.clone(), timeout, tor));
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(2));
        self.active.fetch_sub(1, Ordering::SeqCst);
        let status = self
            .statuses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(200))?;
        let body=self.body.lock().unwrap().clone().unwrap_or_else(|| {
            if url.ends_with("/ip") { b"{\"origin\":\"127.0.0.1\"}".to_vec() }
            else if url.contains("rss") { b"<rss><channel><item><link>https://protext.cz/zprava/3</link></item></channel></rss>".to_vec() }
            else {format!("<h1>{}</h1>{}","Long article title ".repeat(4),include_str!("../fixtures/article.html")).into_bytes()}
        });
        let mut headers = HeaderMap::new();
        if let Some(value) = self.retry_after.lock().unwrap().as_ref() {
            headers.insert("retry-after", value.parse().unwrap());
        }
        Ok(Response {
            status,
            headers,
            body,
        })
    }
    fn sleep(&self, seconds: f64) {
        self.sleeps.lock().unwrap().push(seconds);
    }
    fn random(&self, low: f64, high: f64) -> f64 {
        (low + high) / 2.0
    }
    fn renew(&self) -> bool {
        self.renewals.fetch_add(1, Ordering::SeqCst);
        true
    }
}
pub fn network() -> (Network, Arc<Mock>) {
    let m = Arc::new(Mock::default());
    (
        Network {
            transport: m.clone(),
            ..Network::default()
        },
        m,
    )
}
