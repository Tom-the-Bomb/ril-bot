use reqwest::IntoUrl;
use super::resolver::Error;

pub async fn url_to_bytes<T: IntoUrl>(url: T) -> Result<Vec<u8>, Error> {
    let result = reqwest::get(url)
        .await?;

    if result.status().is_success() {
        Ok(result
            .bytes()
            .await?
            .into())
    } else {
        Err(Error::FetchUrlError)
    }
}