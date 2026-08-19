//! The kendex.ai community directory and the skills.sh search surface,
//! read the way the app reads any remote: strictly parsed, capped, cached
//! with an ETag, and honest about staleness when the network is away.

pub mod cache;
pub mod index;
pub mod skillssh;
pub mod view;

use crate::error::{CoreError, Result};
use crate::process::Hardened;
use std::time::Duration;

/// Where the directory lives. One environment variable moves every read
/// (previews, tests); nothing else configures it.
pub fn base_url() -> String {
    std::env::var("KENDEX_API").unwrap_or_else(|_| "https://kendex.ai".into())
}

pub struct FetchResponse {
    pub status: u16,
    pub etag: Option<String>,
    pub body: Vec<u8>,
}

/// The one seam registry reads go through — tests hand in a canned
/// transport, production hands in curl.
pub trait Fetch {
    fn get(&self, url: &str, if_none_match: Option<&str>) -> Result<FetchResponse>;
}

/// curl under the hardened runner: TLS and proxy configuration come from
/// the system, arguments stay data. Plain http is honored only when the
/// caller's URL asks for it explicitly (a local override), never chosen.
pub struct CurlFetch;

impl Fetch for CurlFetch {
    fn get(&self, url: &str, if_none_match: Option<&str>) -> Result<FetchResponse> {
        let proto = if url.starts_with("http://") {
            "=http"
        } else {
            "=https"
        };
        let mut args: Vec<String> = vec![
            "-sS".into(),
            "-i".into(),
            "--max-time".into(),
            "20".into(),
            "--proto".into(),
            proto.into(),
            "--max-filesize".into(),
            "16000000".into(),
        ];
        if let Some(etag) = if_none_match {
            args.push("-H".into());
            args.push(format!("If-None-Match: {etag}"));
        }
        args.push("--".into());
        args.push(url.into());
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let output = Hardened::curl(&arg_refs)
            .timeout(Duration::from_secs(25))
            .run()?;
        if !output.status.success() {
            return Err(CoreError::RegistryUnavailable {
                why: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        parse_http_response(&output.stdout)
    }
}

/// Split `curl -i` output into the final status, ETag and body. Redirects
/// stack header blocks, so blocks are consumed while the remainder still
/// opens like a response.
fn parse_http_response(raw: &[u8]) -> Result<FetchResponse> {
    let mut rest = raw;
    let mut status = 0u16;
    let mut etag = None;
    while rest.starts_with(b"HTTP/") {
        let Some(end) = find_blank_line(rest) else {
            break;
        };
        let head = String::from_utf8_lossy(&rest[..end]);
        let mut lines = head.lines();
        if let Some(code) = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse::<u16>().ok())
        {
            status = code;
        }
        etag = lines
            .filter_map(|line| line.split_once(':'))
            .find(|(name, _)| name.eq_ignore_ascii_case("etag"))
            .map(|(_, value)| value.trim().to_string());
        rest = &rest[end..];
        while rest.starts_with(b"\r\n") || rest.starts_with(b"\n") {
            rest = if rest.starts_with(b"\r\n") {
                &rest[2..]
            } else {
                &rest[1..]
            };
        }
        // 100-continue and redirect bodies are empty; the next block, if
        // any, opens immediately with its own status line.
    }
    if status == 0 {
        return Err(CoreError::RegistryMalformed {
            why: "response carried no HTTP status line".into(),
        });
    }
    Ok(FetchResponse {
        status,
        etag,
        body: rest.to_vec(),
    })
}

fn find_blank_line(raw: &[u8]) -> Option<usize> {
    raw.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|at| at + 2)
        .or_else(|| {
            raw.windows(2)
                .position(|window| window == b"\n\n")
                .map(|at| at + 1)
        })
}
