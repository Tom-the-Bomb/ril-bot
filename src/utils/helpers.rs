use reqwest::{Client, IntoUrl};
use super::error::Error;

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