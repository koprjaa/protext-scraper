// Project: protext-scraper
// File: tests/cli.rs
// Description: Exercise the installed-style executable against a local SOCKS stub.
// Author: Jan Alexandr Kopřiva
// License: MIT
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
#[test]
fn unavailable_tor_exits_zero_and_ignores_arguments() {
    let listener = TcpListener::bind("127.0.0.1:9050")
        .expect("The CLI fixture requires the default Tor port to be free");
    listener.set_nonblocking(true).unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        let mut requests = 0;
        while requests < 2 && Instant::now() < deadline {
            let (mut s, _) = match listener.accept() {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(e) => panic!("{e}"),
            };
            s.set_nonblocking(false).unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            s.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut prefix = [0; 2];
            s.read_exact(&mut prefix).unwrap();
            assert_eq!(prefix[0], 5);
            let mut methods = vec![0; prefix[1] as usize];
            s.read_exact(&mut methods).unwrap();
            s.write_all(&[5, 0]).unwrap();
            let mut prefix = [0; 4];
            s.read_exact(&mut prefix).unwrap();
            assert_eq!(&prefix[..3], &[5, 1, 0]);
            match prefix[3] {
                3 => {
                    let mut len = [0];
                    s.read_exact(&mut len).unwrap();
                    let mut target = vec![0; len[0] as usize + 2];
                    s.read_exact(&mut target).unwrap();
                    assert!(target.starts_with(b"httpbin.org"));
                }
                1 => {
                    let mut target = [0; 6];
                    s.read_exact(&mut target).unwrap();
                }
                _ => panic!("Unexpected SOCKS address type"),
            }
            s.write_all(&[5, 1, 0, 1, 0, 0, 0, 0, 0, 0]).unwrap();
            s.shutdown(Shutdown::Write).unwrap();
            let mut rest = Vec::new();
            let _ = s.read_to_end(&mut rest);
            requests += 1;
        }
        assert_eq!(requests, 2);
    });
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("data")).unwrap();
    std::fs::write(dir.path().join("data/ascii.txt"), "LOCAL ART").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_protext-scraper"))
        .args(["--ignored", "extra"])
        .current_dir(dir.path())
        .env("PATH", dir.path().join("missing-programs"))
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    server.join().unwrap();
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/cli_reference.json")).unwrap();
    assert_eq!(
        result.status.code().map(i64::from),
        reference["exit_code"].as_i64()
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(stdout.starts_with("LOCAL ART\n\n"));
    assert!(stdout.contains("Checking Tor connection..."));
    assert!(stdout.contains("Starting Tor service..."));
    assert!(stdout.contains("Tor is not available. Please install Tor:"));
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        reference["stderr"].as_str().unwrap()
    );
    for message in [
        "Checking Tor connection...",
        "Attempting to start Tor service...",
        "Tor is not available. Please install Tor:",
    ] {
        assert!(stdout.contains(message));
        assert!(reference["stdout"].as_str().unwrap().contains(message));
    }
    assert!(!dir.path().join("output").exists());
}
