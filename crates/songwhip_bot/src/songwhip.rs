// use crate::pagerduty::models::OncallList;
use anyhow::{anyhow, bail, Result};
use hyper::client::{Client, HttpConnector};
use hyper::{Body, Request, StatusCode};
use hyper_rustls::{ConfigBuilderExt, HttpsConnector, HttpsConnectorBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::OnceCell;
use tracing::debug;

const SONGWHIP_URL: &str = "https://songwhip.com/";
static SW_CLIENT: OnceCell<Client<HttpsConnector<HttpConnector>>> = OnceCell::const_new();
pub async fn get_or_init_songwhip_client() -> &'static Client<HttpsConnector<HttpConnector>> {
    SW_CLIENT
        .get_or_init(|| async { new_songwhip_client() })
        .await
}

pub fn new_songwhip_client() -> Client<HttpsConnector<HttpConnector>> {
    let https = HttpsConnectorBuilder::new()
        .with_tls_config(
            rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_native_roots()
                .with_no_client_auth(),
        )
        .https_or_http()
        .enable_http1()
        .build();

    Client::builder().build::<_, Body>(https)
}

pub fn build_songwhip_request(url: &str) -> Request<Body> {
    Request::builder()
        .uri(SONGWHIP_URL)
        .method("POST")
        .body(json!({ "url": url }).to_string().into())
        .unwrap()
}

pub async fn songwhip_query(data: &str) -> Result<Option<SongwhipResponseBody>> {
    let response = get_or_init_songwhip_client()
        .await
        .request(build_songwhip_request(data))
        .await?;

    let status = &response.status();
    let body_bytes = hyper::body::to_bytes(response.into_body()).await?;

    if status.is_success() {
        let formatted_response: SongwhipResponseBody = serde_json::from_slice(&body_bytes)?;
        Ok(Some(formatted_response))
    } else {
        match *status {
            StatusCode::BAD_REQUEST => {
                debug!("No song found for url: {}", data);
                Ok(None)
            }
            _ => {
                bail!("Error from Songwhip: {} - {:?}", status, body_bytes)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SongwhipResponseBody {
    pub name: String,
    pub url: String,
    pub image: Option<String>,
    pub artists: Vec<SongwhipArtist>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SongwhipArtist {
    pub name: String,
    pub description: Option<String>,
    pub image: Option<String>,
}
