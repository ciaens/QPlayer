//! Covers kept on disk once fetched, keyed by their URL, so that a restart shows them without the network.

use std::{
    fmt::Write,
    path::{Path, PathBuf},
    sync::RwLock,
};

use reqwest::header::CONTENT_TYPE;
use sha2::{Digest, Sha256};

use crate::{AppResult, downloader::expand_tilde, error::PlayerError};

/// The `covers` folder of the audio cache directory, following it when it changes.
pub struct Covers {
    directory: RwLock<PathBuf>,
    http_client: reqwest::Client,
}

impl Covers {
    #[must_use]
    pub fn new(cache_directory: &Path) -> Self {
        Self {
            directory: RwLock::new(expand_tilde(cache_directory)),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn set_directory(&self, cache_directory: &Path) -> AppResult<()> {
        *self.directory.write()? = expand_tilde(cache_directory);
        Ok(())
    }

    /// The image at `url`, from disk when it was fetched before.
    pub async fn get(&self, url: &str) -> AppResult<Vec<u8>> {
        let folder = self.directory.read()?.join("covers");
        let path = folder.join(file_name(url));
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(bytes);
        }
        let response = self.http_client.get(url).send().await?.error_for_status()?;
        let image = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("image/"));
        let bytes = response.bytes().await?;
        if !image || bytes.is_empty() {
            return Err(PlayerError::Client {
                message: format!("{url} is not an image"),
            });
        }
        if let Err(err) = keep(&folder, &path, &bytes).await {
            tracing::warn!("Not keeping the cover {url}: {err}");
        }
        Ok(bytes.to_vec())
    }
}

/// Written to a side file of this process first, so that a cover on disk is always whole.
async fn keep(folder: &Path, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    tokio::fs::create_dir_all(folder).await?;
    let partial = path.with_extension(format!("{}.part", std::process::id()));
    tokio::fs::write(&partial, bytes).await?;
    if let Err(err) = tokio::fs::rename(&partial, path).await {
        let _ = tokio::fs::remove_file(partial).await;
        return Err(err);
    }
    Ok(())
}

fn file_name(url: &str) -> String {
    Sha256::digest(url)
        .iter()
        .fold(String::new(), |mut name, byte| {
            let _ = write!(name, "{byte:02x}");
            name
        })
}
