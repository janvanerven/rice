use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;

const UNSPLASH_API: &str = "https://api.unsplash.com";
const ALLOWED_HOSTS: &[&str] = &["images.unsplash.com", "plus.unsplash.com"];
const MAX_DOWNLOAD_SIZE: usize = 5 * 1024 * 1024; // 5MB
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Deserialize)]
pub struct UnsplashSearchResponse {
    pub results: Vec<UnsplashPhoto>,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashPhoto {
    pub id: String,
    pub urls: UnsplashUrls,
    pub user: UnsplashUser,
    pub links: UnsplashLinks,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUrls {
    pub regular: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUser {
    pub name: String,
    pub links: UnsplashUserLinks,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashUserLinks {
    pub html: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsplashLinks {
    pub html: String,
    pub download_location: String,
}

/// Search Unsplash for a landscape photo matching the query.
pub async fn search(
    client: &Client,
    access_key: &str,
    query: &str,
) -> Result<Option<UnsplashPhoto>, String> {
    let resp = client
        .get(format!("{UNSPLASH_API}/search/photos"))
        .header("Authorization", format!("Client-ID {access_key}"))
        .query(&[
            ("query", query),
            ("per_page", "1"),
            ("orientation", "landscape"),
        ])
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("Unsplash search failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        return Err(format!("Unsplash API returned {status}"));
    }

    let data: UnsplashSearchResponse = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse Unsplash response: {e}"))?;

    Ok(data.results.into_iter().next())
}

/// Download a photo to a temp file. Returns (temp_path, final_filename).
/// Caller must call `finalize()` after DB commit, or `cleanup_temp()` on failure.
pub async fn download(
    client: &Client,
    photo: &UnsplashPhoto,
    dir: &Path,
) -> Result<(PathBuf, String), String> {
    // SSRF protection: verify download URL host
    let url = &photo.urls.regular;
    let parsed = reqwest::Url::parse(url).map_err(|e| format!("Invalid photo URL: {e}"))?;
    let host = parsed.host_str().unwrap_or("");
    if !ALLOWED_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
    {
        return Err(format!("Blocked download from untrusted host: {host}"));
    }

    let resp = client
        .get(url)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| format!("Failed to download photo: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Photo download returned {}", resp.status()));
    }

    let filename = format!("{}.jpg", ulid::Ulid::new());
    let tmp_filename = format!("{filename}.tmp");
    let tmp_path = dir.join(&tmp_filename);

    fs::create_dir_all(dir)
        .await
        .map_err(|e| format!("Failed to create upload dir: {e}"))?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read photo data: {e}"))?;

    if bytes.len() > MAX_DOWNLOAD_SIZE {
        return Err("Downloaded photo exceeds 5MB size limit".into());
    }

    let mut file = fs::File::create(&tmp_path)
        .await
        .map_err(|e| format!("Failed to create temp file: {e}"))?;

    file.write_all(&bytes).await.map_err(|e| {
        let tmp = tmp_path.clone();
        tokio::spawn(async move {
            let _ = fs::remove_file(tmp).await;
        });
        format!("Failed to write photo: {e}")
    })?;

    file.flush()
        .await
        .map_err(|e| format!("Failed to flush: {e}"))?;

    Ok((tmp_path, filename))
}

/// Rename temp file to final path. Call after DB commit.
pub async fn finalize(tmp_path: &Path, final_name: &str) -> Result<PathBuf, String> {
    let final_path = tmp_path.parent().unwrap().join(final_name);
    fs::rename(tmp_path, &final_path)
        .await
        .map_err(|e| format!("Failed to finalize file: {e}"))?;
    Ok(final_path)
}

/// Clean up a temp file on failure.
pub async fn cleanup_temp(tmp_path: &Path) {
    let _ = fs::remove_file(tmp_path).await;
}

/// Trigger Unsplash download tracking (required by TOS). Fire-and-forget.
pub fn track_download(client: Client, access_key: String, photo: &UnsplashPhoto) {
    let url = photo.links.download_location.clone();
    tokio::spawn(async move {
        let result = client
            .get(&url)
            .header("Authorization", format!("Client-ID {access_key}"))
            .timeout(Duration::from_secs(5))
            .send()
            .await;
        match result {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => {
                tracing::error!("Unsplash download tracking returned {}", resp.status());
            }
            Err(e) => {
                tracing::error!("Unsplash download tracking failed: {e}");
            }
        }
    });
}
