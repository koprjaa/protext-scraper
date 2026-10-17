// Project: protext-scraper
// File: tests/network.rs
// Description: Offline HTTP, retry, encoding, and Tor protocol tests.
// Author: Jan Alexandr Kopřiva
// License: MIT
mod support;
use protext_scraper::network::*;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, TcpListener},
    sync::atomic::Ordering,
    thread,
};
#[test]
fn retry_statuses_waits_and_headers() {
    let (n, m) = support::network();
    *m.statuses.lock().unwrap() = [Ok(429), Ok(403), Ok(200)].into();
    *m.retry_after.lock().unwrap() = Some("7".into());
    assert!(n.request("http://fixture.test", 3, 1.0, true).is_some());
    assert_eq!(*m.sleeps.lock().unwrap(), [0.125, 7.0, 0.125, 15.0, 0.125]);
    assert_eq!(m.renewals.load(Ordering::SeqCst), 2);
    for (_, h, t, tor) in m.requests.lock().unwrap().iter() {
        assert!(*tor);
        assert_eq!(*t, 16.0);
        assert_eq!(h.len(), 15);
        assert!(USER_AGENTS.contains(&h["user-agent"].to_str().unwrap()));
        assert_eq!(h["accept-language"], "en-US,en;q=0.7");
        assert_eq!(h["accept-encoding"], "gzip, deflate, br");
        assert_eq!(h["sec-ch-ua-platform"], "\"Windows\"");
    }
    assert_eq!(USER_AGENTS.len(), 21);
}
#[test]
fn service_unavailable_renews_on_second_attempt() {
    let (n, m) = support::network();
    *m.statuses.lock().unwrap() = [Ok(503), Ok(503), Ok(503)].into();
    assert!(n.request("http://fixture.test", 3, 1.0, true).is_none());
    assert_eq!(m.renewals.load(Ordering::SeqCst), 2);
    assert_eq!(
        *m.sleeps.lock().unwrap(),
        [0.125, 15.0, 0.125, 15.0, 0.125, 15.0]
    );
}
#[test]
fn errors_exponential_backoff_and_attempt_limits() {
    let (n, m) = support::network();
    *m.statuses.lock().unwrap() = [Err("timeout".into()), Ok(404), Err("closed".into())].into();
    assert!(n.request("http://fixture.test", 3, 1.0, true).is_none());
    assert_eq!(*m.sleeps.lock().unwrap(), [0.125, 6.0, 0.125, 7.0, 0.125]);
    assert_eq!(m.renewals.load(Ordering::SeqCst), 0);
    assert_eq!(m.requests.lock().unwrap().len(), 3);
    assert!(n.request("http://fixture.test", 0, 1.0, false).is_none());
}
#[test]
fn malformed_retry_after_uses_default_without_crashing() {
    let (n, m) = support::network();
    *m.statuses.lock().unwrap() = [Ok(429), Ok(200)].into();
    *m.retry_after.lock().unwrap() = Some("invalid".into());
    assert!(n.request("http://fixture.test", 2, 1.0, false).is_some());
    assert_eq!(m.sleeps.lock().unwrap()[1], 180.0);
    assert_eq!(m.renewals.load(Ordering::SeqCst), 0);
}
#[test]
fn persistent_errors_and_rss_two_attempts() {
    let (n, m) = support::network();
    *m.statuses.lock().unwrap() =
        [Err("x".into()), Err("x".into()), Err("x".into()), Ok(200)].into();
    assert!(n.request("http://fixture.test", 4, 0.5, true).is_some());
    assert_eq!(m.renewals.load(Ordering::SeqCst), 1);
    let (n, m) = support::network();
    *m.statuses.lock().unwrap() = [Ok(500), Ok(200)].into();
    assert_eq!(n.fetch_latest_rss_articles(), Some((3, 3)));
    assert_eq!(m.requests.lock().unwrap().len(), 2);
    *m.body.lock().unwrap() = Some(b"<rss/>".to_vec());
    assert_eq!(n.fetch_latest_rss_articles(), None);
}
#[test]
fn encoding_utf8_and_windows1250() {
    let text = "Příliš žluťoučký kůň úpěl ďábelské ódy. ".repeat(10);
    let (bytes, _, errors) = encoding_rs::WINDOWS_1250.encode(&text);
    assert!(!errors);
    assert_eq!(decode(&bytes), text);
    assert_eq!(decode(text.as_bytes()), text);
    let (n, m) = support::network();
    let source = format!("<h1>{text}</h1><div class='fulltext'>{text}</div>");
    let (bytes, _, _) = encoding_rs::WINDOWS_1250.encode(&source);
    *m.body.lock().unwrap() = Some(bytes.into_owned());
    assert_eq!(n.fetch_article_by_id(1).unwrap()["content"], text.trim());
}
#[test]
fn full_content_uses_direct_request_and_original_agent() {
    let (n, m) = support::network();
    assert!(n.fetch_full_content("http://fixture.test").is_some());
    let req = m.requests.lock().unwrap();
    assert!(!req[0].3);
    assert_eq!(req[0].2, 15.0);
    assert!(req[0].1["user-agent"]
        .to_str()
        .unwrap()
        .contains("Chrome/91.0.4472.124"));
}
fn close(mut s: std::net::TcpStream) {
    s.shutdown(Shutdown::Write).unwrap();
    let mut bytes = Vec::new();
    let _ = s.read_to_end(&mut bytes);
}
#[test]
fn actual_http_transport_redirect_and_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for i in 0..2 {
            let (mut s, _) = listener.accept().unwrap();
            s.set_nonblocking(false).unwrap();
            s.set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(s.try_clone().unwrap());
            let mut request = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                request.push_str(&line);
            }
            assert!(request
                .to_lowercase()
                .contains("accept-language: en-us,en;q=0.7"));
            let response = if i == 0 {
                format!("HTTP/1.1 302 Found\r\nLocation: http://{address}/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            } else {
                "HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nfixture".into()
            };
            s.write_all(response.as_bytes()).unwrap();
            drop(reader);
            close(s);
        }
    });
    let r = HttpTransport
        .get(
            &format!("http://{address}/start"),
            &browser_headers(),
            5.0,
            false,
        )
        .unwrap();
    assert_eq!(r.status, 200);
    assert_eq!(r.body, b"fixture");
    server.join().unwrap();
}
#[test]
fn tor_control_authentication_and_newnym() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_nonblocking(false).unwrap();
        let mut reader = BufReader::new(s.try_clone().unwrap());
        for expected in ["AUTHENTICATE \"\"\r\n", "SIGNAL NEWNYM\r\n"] {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line, expected);
            s.write_all(b"250 OK\r\n").unwrap();
        }
        drop(reader);
        close(s);
    });
    assert!(renew_at(address).unwrap());
    server.join().unwrap();
}
#[test]
fn tor_authentication_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut s, _) = listener.accept().unwrap();
        s.set_nonblocking(false).unwrap();
        let mut reader = BufReader::new(s.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        s.write_all(b"515 Authentication failed\r\n").unwrap();
        drop(reader);
        close(s);
    });
    assert!(!renew_at(address).unwrap());
    server.join().unwrap();
}
#[test]
fn startup_check_status_and_bad_json() {
    let (n, m) = support::network();
    assert!(n.check_tor_connection());
    *m.statuses.lock().unwrap() = [Ok(403)].into();
    assert!(!n.check_tor_connection());
    *m.body.lock().unwrap() = Some(b"broken".to_vec());
    assert!(!n.check_tor_connection());
}
