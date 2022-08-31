//! contains various frequently used small, general helper functions

use reqwest::{Client, IntoUrl};
use super::error::Error;


/// a helper function to fetch the bytes of a provided url
/// does not implement checks such as for content type or length, as we will assume it is done beforehand
pub async fn url_to_bytes<T: IntoUrl>(client: Option<&Client>, url: T) -> Result<Vec<u8>, Error> {
    let result = if let Some(client) = client {
        client.get(url)
            .send()
            .await
    } else {
        reqwest::get(url)
            .await
    }?;

    if result.status().is_success() {
        Ok(result
            .bytes()
            .await?
            .into())
    } else {
        Err(Error::FetchUrlError)
    }
}

/// helper function that humanizes an integer representing a number of bytes to a human readable formats with SI units
pub fn humanize_bytes(size: u64) -> String {
    let mut size = size as f32;
    let units = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

    for unit in units {
        if size < 1024.0 {
            return format!("{:.2} {}", size, unit);
        }

        size /= 1024.0;
    }

    "NaN".to_string()
}