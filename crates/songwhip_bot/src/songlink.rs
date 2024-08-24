use std::collections::HashMap;

// use crate::pagerduty::models::OncallList;
use anyhow::{bail, Result};
use hyper::client::{Client, HttpConnector};
use hyper::{Body, Request, StatusCode};
use hyper_rustls::{ConfigBuilderExt, HttpsConnector, HttpsConnectorBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::OnceCell;
use tracing::debug;
use url::Url;

const SONGLINK_URL: &str = "https://api.song.link/v1-alpha.1/links";
static SL_CLIENT: OnceCell<Client<HttpsConnector<HttpConnector>>> = OnceCell::const_new();
pub async fn get_or_init_songlink_client() -> &'static Client<HttpsConnector<HttpConnector>> {
    SL_CLIENT
        .get_or_init(|| async { new_songlink_client() })
        .await
}

pub fn new_songlink_client() -> Client<HttpsConnector<HttpConnector>> {
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

pub fn build_songlink_request(url: &str) -> Request<Body> {
    let formatted_url =
        url::Url::parse_with_params(SONGLINK_URL, [("url", url)]).expect("invalid_url");

    Request::builder()
        .uri(formatted_url.as_str())
        .method("GET")
        .body("".into())
        .unwrap()
}

pub async fn songlink_query(data: &str) -> Result<Option<SonglinkResponseBody>> {
    let response = get_or_init_songlink_client()
        .await
        .request(build_songlink_request(data))
        .await?;

    let status = &response.status();
    let body_bytes = hyper::body::to_bytes(response.into_body()).await?;

    if status.is_success() {
        let formatted_response: SonglinkResponseBody = serde_json::from_slice(&body_bytes)?;
        Ok(Some(formatted_response))
    } else {
        match *status {
            StatusCode::BAD_REQUEST => {
                // debug!("No song found for url: {}", data); // removing this line to prevent logging possibly 'sensitive' youtube videos
                debug!("No song found for that url");
                Ok(None)
            }
            _ => {
                bail!("Error from Songwhip: {} - {:?}", status, body_bytes)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SonglinkResponseBody {
    pub page_url: String,
    pub entity_unique_id: String,
    pub entities_by_unique_id: HashMap<String, SonglinkEntity>,
    pub links_by_platform: HashMap<String, SonglinkPlatformLink>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SonglinkEntity {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub title: String,
    pub artist_name: String,
    pub thumbnail_url: String,
    pub thumbnail_width: u64,
    pub thumbnail_height: u64,
    pub api_provider: String,
    pub platforms: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SonglinkPlatformLink {
    pub country: String,
    pub url: String,
    pub entity_unique_id: String,
}

pub fn map_platform_to_formatted_display_name(platform: &str) -> Option<&'static str> {
    match platform {
        "appleMusic" => Some(":apple-inc: _*Apple Music*_"),
        "spotify" => Some(":spotify: _*Spotify*_"),
        "deezer" => Some(":deezer: _*Deezer*_"),
        "youtube" => Some(":youtube: _*Youtube*_"),
        "youtubeMusic" => Some(":youtube-music: _*YT Music*_"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_songlink_request, SonglinkResponseBody};

    const songlink_output: &str = r#"{"entityUniqueId":"ITUNES_SONG::44733632","userCountry":"US","pageUrl":"https://song.link/us/i/44733632","entitiesByUniqueId":{"BOOMPLAY_SONG::20846327":{"id":"20846327","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://source.boomplaymusic.com/group10/M00/04/27/3f8569ae345c41e69423d424a0751ff6_464_464.jpg","thumbnailWidth":464,"thumbnailHeight":464,"apiProvider":"boomplay","platforms":["boomplay"]},"DEEZER_SONG::64497787":{"id":"64497787","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://cdns-images.dzcdn.net/images/cover/22c0cdb3b13212dcadf78823ddb3702b/500x500-000000-80-0-0.jpg","thumbnailWidth":500,"thumbnailHeight":500,"apiProvider":"deezer","platforms":["deezer"]},"ITUNES_SONG::44733632":{"id":"44733632","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://is1-ssl.mzstatic.com/image/thumb/Features114/v4/4e/80/38/4e80381f-d283-ea89-c44e-c8f650fab0c8/dj.plcmkwuf.jpg/512x512bb.jpg","thumbnailWidth":512,"thumbnailHeight":512,"apiProvider":"itunes","platforms":["appleMusic","itunes"]},"NAPSTER_SONG::tra.7345970":{"id":"tra.7345970","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://direct.rhapsody.com/imageserver/images/alb.7338556/385x385.jpeg","thumbnailWidth":385,"thumbnailHeight":385,"apiProvider":"napster","platforms":["napster"]},"PANDORA_SONG::TR:5831794":{"id":"TR:5831794","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://content-images.p-cdn.com/images/14/88/28/46/55ce4f52ad6940fdefe248b9/_500W_500H.jpg","thumbnailWidth":500,"thumbnailHeight":500,"apiProvider":"pandora","platforms":["pandora"]},"SPOTIFY_SONG::12Pgnvye9Vn1X5e9fAzBiG":{"id":"12Pgnvye9Vn1X5e9fAzBiG","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://i.scdn.co/image/ab67616d0000b273a67147d2906c72fd60850747","thumbnailWidth":640,"thumbnailHeight":640,"apiProvider":"spotify","platforms":["spotify"]},"TIDAL_SONG::31448515":{"id":"31448515","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://resources.tidal.com/images/4c5f7148/65ac/4c1f/a5b3/4fccf0032c26/640x640.jpg","thumbnailWidth":640,"thumbnailHeight":640,"apiProvider":"tidal","platforms":["tidal"]},"YOUTUBE_VIDEO::SZsvRgqi3Fc":{"id":"SZsvRgqi3Fc","type":"song","title":"What We Worked For","artistName":"Against Me! - Topic","thumbnailUrl":"https://i.ytimg.com/vi/SZsvRgqi3Fc/hqdefault.jpg","thumbnailWidth":480,"thumbnailHeight":360,"apiProvider":"youtube","platforms":["youtube","youtubeMusic"]}},"linksByPlatform":{"boomplay":{"country":"US","url":"https://www.boomplay.com/songs/20846327","entityUniqueId":"BOOMPLAY_SONG::20846327"},"deezer":{"country":"US","url":"https://www.deezer.com/track/64497787","entityUniqueId":"DEEZER_SONG::64497787"},"napster":{"country":"US","url":"https://play.napster.com/track/tra.7345970","entityUniqueId":"NAPSTER_SONG::tra.7345970"},"pandora":{"country":"US","url":"https://www.pandora.com/TR:5831794","entityUniqueId":"PANDORA_SONG::TR:5831794"},"spotify":{"country":"US","url":"https://open.spotify.com/track/12Pgnvye9Vn1X5e9fAzBiG","nativeAppUriDesktop":"spotify:track:12Pgnvye9Vn1X5e9fAzBiG","entityUniqueId":"SPOTIFY_SONG::12Pgnvye9Vn1X5e9fAzBiG"},"tidal":{"country":"US","url":"https://listen.tidal.com/track/31448515","entityUniqueId":"TIDAL_SONG::31448515"},"youtube":{"country":"US","url":"https://www.youtube.com/watch?v=SZsvRgqi3Fc","entityUniqueId":"YOUTUBE_VIDEO::SZsvRgqi3Fc"},"youtubeMusic":{"country":"US","url":"https://music.youtube.com/watch?v=SZsvRgqi3Fc","entityUniqueId":"YOUTUBE_VIDEO::SZsvRgqi3Fc"},"appleMusic":{"country":"US","url":"https://geo.music.apple.com/us/album/_/44734006?i=44733632&mt=1&app=music&ls=1&at=1000lHKX&ct=api_http&itscg=30200&itsct=odsl_m","nativeAppUriMobile":"music://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=music&ls=1&at=1000lHKX&ct=api_uri_m&itscg=30200&itsct=odsl_m","nativeAppUriDesktop":"itmss://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=music&ls=1&at=1000lHKX&ct=api_uri_d&itscg=30200&itsct=odsl_m","entityUniqueId":"ITUNES_SONG::44733632"},"itunes":{"country":"US","url":"https://geo.music.apple.com/us/album/_/44734006?i=44733632&mt=1&app=itunes&ls=1&at=1000lHKX&ct=api_http&itscg=30200&itsct=odsl_m","nativeAppUriMobile":"itmss://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=itunes&ls=1&at=1000lHKX&ct=api_uri_m&itscg=30200&itsct=odsl_m","nativeAppUriDesktop":"itmss://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=itunes&ls=1&at=1000lHKX&ct=api_uri_d&itscg=30200&itsct=odsl_m","entityUniqueId":"ITUNES_SONG::44733632"}}}"#;

    #[test]
    fn test_songlink_url() {
        let output =
            build_songlink_request("https://music.apple.com/us/song/what-we-worked-for/44733632");

        assert_eq!(output.uri(), "https://api.song.link/v1-alpha.1/links?url=https%3A%2F%2Fmusic.apple.com%2Fus%2Fsong%2Fwhat-we-worked-for%2F44733632")
    }

    #[test]
    fn test_deserialize_songlink_body() {
        let _res: SonglinkResponseBody = serde_json::from_str(songlink_output).unwrap();
    }
}
