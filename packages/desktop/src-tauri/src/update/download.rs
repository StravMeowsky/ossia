//! Streaming asset downloads with checksum computation.

use super::{fetch, Error, Result};
use reqwest::{
    blocking::Response,
    header::{HeaderMap, LOCATION},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

pub fn to_file(
    url: &str,
    token: Option<&str>,
    dest: &Path,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<String> {
    let mut response = asset_response(url, token, DOWNLOAD_TIMEOUT)?;
    let total = response.content_length().unwrap_or(0);
    let mut file = File::create(dest)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut written = 0u64;
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| Error::other(format!("download: {error}")))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        written += read as u64;
        progress(written, total);
    }
    file.flush()?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub fn text(url: &str, token: Option<&str>) -> Result<String> {
    let response = asset_response(url, token, Duration::from_secs(30))?;
    let mut body = String::new();
    response
        .take(1024 * 1024)
        .read_to_string(&mut body)
        .map_err(|error| Error::other(format!("download: {error}")))?;
    Ok(body)
}

fn asset_response(url: &str, token: Option<&str>, timeout: Duration) -> Result<Response> {
    fetch::ensure_secure(url)?;
    let client = fetch::no_redirect_client()?;
    let response = fetch::asset_get(&client, url, token)?
        .timeout(timeout)
        .send()
        .map_err(|error| Error::other(format!("download: {error}")))?;
    let status = response.status().as_u16();
    if (300..400).contains(&status) {
        let location = redirect_location(url, response.headers())?;
        fetch::ensure_secure(&location)?;
        let response = fetch::client()?
            .get(location)
            .timeout(timeout)
            .send()
            .map_err(|error| Error::other(format!("download: {error}")))?;
        return success(response);
    }
    success(response)
}

fn success(response: Response) -> Result<Response> {
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(response);
    }
    let headers = response.headers().clone();
    let response = response;
    let mut body = String::new();
    let _ = response.take(8 * 1024).read_to_string(&mut body);
    Err(Error::other(format!(
        "download: {}",
        super::github::describe_failure(status, &headers, &body)
    )))
}

fn redirect_location(url: &str, headers: &HeaderMap) -> Result<String> {
    let location = headers
        .get(LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .ok_or_else(|| Error::other("download redirect has no location"))?;
    reqwest::Url::parse(url)
        .and_then(|base| base.join(&location))
        .map(|url| url.to_string())
        .map_err(|error| Error::other(format!("download redirect: {error}")))
}
