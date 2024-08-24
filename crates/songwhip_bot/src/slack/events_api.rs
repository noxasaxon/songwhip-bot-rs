use super::SlackStateWorkaround;
use crate::songlink::{
    map_platform_to_formatted_display_name, songlink_query, SonglinkResponseBody,
};
use axum::{
    body::{self},
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use itertools::Itertools;
use serde_json::{to_value, Value};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tracing::{error, info};

pub async fn axum_handler_slack_events_api(
    Extension(slack_state): Extension<Arc<SlackStateWorkaround>>,
    Json(payload): Json<SlackPushEvent>,
) -> impl IntoResponse {
    let (status, body) = handle_slack_event(slack_state, payload).await;

    Response::builder()
        .status(status)
        .header("X-Slack-No-Retry", "1")
        .body(body::boxed(body::Full::new(body.to_string().into())))
        .unwrap()
}

pub async fn handle_slack_event(
    slack_state: Arc<SlackStateWorkaround>,
    payload: SlackPushEvent,
) -> (StatusCode, Value) {
    match payload {
        SlackPushEvent::EventCallback(event_req) => {
            match event_req.event {
                SlackEventCallbackBody::LinkShared(event) => {
                    if event.is_bot_user_member {
                        let msg_urls: Vec<String> = event
                            .links
                            .into_iter()
                            .map(|link_obj| link_obj.url)
                            .collect();

                        process_urls_and_post_songlink_message(
                            msg_urls,
                            slack_state,
                            event.channel,
                            event.message_ts,
                        );
                    }
                }

                _ => info!("unhandled event sub type"),
            }
            (StatusCode::OK, Value::default())
        }
        SlackPushEvent::UrlVerification(url_verify_req) => {
            (StatusCode::OK, to_value(url_verify_req).unwrap())
        }
        SlackPushEvent::AppRateLimited(rate_limit_req) => {
            // TODO: handle rate limits
            (StatusCode::OK, to_value(rate_limit_req).unwrap())
        }
    }
}

pub fn process_urls_and_post_songlink_message(
    msg_urls: Vec<String>,
    slack_state: Arc<SlackStateWorkaround>,
    slack_channel_id: SlackChannelId,
    msg_timestamp: SlackTs,
) {
    tokio::spawn(async move {
        let mut valid_results = Vec::default();

        for msg_url in msg_urls {
            match songlink_query(&msg_url).await {
                Ok(query) => {
                    if let Some(songwhip_body) = query {
                        valid_results.push(songwhip_body);
                    }
                }
                Err(_) => todo!(""),
            }
        }

        if let Err(slack_err) = slack_state
            .open_session()
            .chat_post_message(
                &SlackApiChatPostMessageRequest::new(
                    slack_channel_id,
                    build_songlink_slack_message(valid_results),
                )
                .with_thread_ts(msg_timestamp)
                .opt_unfurl_links(Some(false))
                .opt_unfurl_media(Some(false)),
            )
            .await
        {
            error!("{}", slack_err);
        }
    });
}

pub fn build_songlink_slack_message(
    sw_responses: Vec<SonglinkResponseBody>,
) -> SlackMessageContent {
    let song_sections: Vec<Vec<SlackBlock>> = sw_responses
        .iter()
        .map(|sl_resp| {
            vec![
                build_songlink_main_block(&sl_resp).into(),
                build_songlink_direct_links_block(&sl_resp).into(),
            ]
        })
        .collect();

    SlackMessageContent::new().with_blocks(song_sections.concat())
}

pub fn build_songlink_full_msg(sl_resp: &SonglinkResponseBody) -> Vec<SlackBlock> {
    vec![
        build_songlink_main_block(&sl_resp).into(),
        build_songlink_direct_links_block(&sl_resp).into(),
    ]
}

pub fn build_songlink_main_block(sl_resp: &SonglinkResponseBody) -> SlackSectionBlock {
    let sample_entity = sl_resp
        .entities_by_unique_id
        .values()
        .by_ref()
        .next()
        .unwrap();

    let artist_name = &sample_entity.artist_name;
    let title = &sample_entity.title;
    let thumbnail_url = &sample_entity.thumbnail_url;

    let section = SlackSectionBlock::new().with_text(md!(format!(
        "<{}|_*{}*_> \n by {}",
        sl_resp.page_url.clone(),
        title,
        artist_name
    )));

    section.with_accessory(SlackSectionBlockElement::Image(
        SlackBlockImageElement::new(thumbnail_url.clone(), "songlink song image".into()),
    ))
}

pub fn build_songlink_direct_links_block(sl_resp: &SonglinkResponseBody) -> SlackSectionBlock {
    let section = SlackSectionBlock::new().with_fields(
        sl_resp
            .links_by_platform
            .iter()
            .filter(|(platform, _l)| map_platform_to_formatted_display_name(platform).is_some())
            .sorted_by_key(|x| x.0)
            .map(|(platform, link_obj)| {
                md!(format!(
                    "<{}|{}>",
                    link_obj.url,
                    map_platform_to_formatted_display_name(&platform).unwrap()
                ))
            })
            .collect(),
    );

    section
}

#[cfg(test)]
mod tests {
    use crate::{events_api::build_songlink_full_msg, write_serde_struct_to_file};

    use super::{build_songlink_main_block, build_songlink_slack_message, SonglinkResponseBody};

    const songlink_output: &str = r#"{"entityUniqueId":"ITUNES_SONG::44733632","userCountry":"US","pageUrl":"https://song.link/us/i/44733632","entitiesByUniqueId":{"BOOMPLAY_SONG::20846327":{"id":"20846327","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://source.boomplaymusic.com/group10/M00/04/27/3f8569ae345c41e69423d424a0751ff6_464_464.jpg","thumbnailWidth":464,"thumbnailHeight":464,"apiProvider":"boomplay","platforms":["boomplay"]},"DEEZER_SONG::64497787":{"id":"64497787","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://cdns-images.dzcdn.net/images/cover/22c0cdb3b13212dcadf78823ddb3702b/500x500-000000-80-0-0.jpg","thumbnailWidth":500,"thumbnailHeight":500,"apiProvider":"deezer","platforms":["deezer"]},"ITUNES_SONG::44733632":{"id":"44733632","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://is1-ssl.mzstatic.com/image/thumb/Features114/v4/4e/80/38/4e80381f-d283-ea89-c44e-c8f650fab0c8/dj.plcmkwuf.jpg/512x512bb.jpg","thumbnailWidth":512,"thumbnailHeight":512,"apiProvider":"itunes","platforms":["appleMusic","itunes"]},"NAPSTER_SONG::tra.7345970":{"id":"tra.7345970","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://direct.rhapsody.com/imageserver/images/alb.7338556/385x385.jpeg","thumbnailWidth":385,"thumbnailHeight":385,"apiProvider":"napster","platforms":["napster"]},"PANDORA_SONG::TR:5831794":{"id":"TR:5831794","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://content-images.p-cdn.com/images/14/88/28/46/55ce4f52ad6940fdefe248b9/_500W_500H.jpg","thumbnailWidth":500,"thumbnailHeight":500,"apiProvider":"pandora","platforms":["pandora"]},"SPOTIFY_SONG::12Pgnvye9Vn1X5e9fAzBiG":{"id":"12Pgnvye9Vn1X5e9fAzBiG","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://i.scdn.co/image/ab67616d0000b273a67147d2906c72fd60850747","thumbnailWidth":640,"thumbnailHeight":640,"apiProvider":"spotify","platforms":["spotify"]},"TIDAL_SONG::31448515":{"id":"31448515","type":"song","title":"What We Worked For","artistName":"Against Me!","thumbnailUrl":"https://resources.tidal.com/images/4c5f7148/65ac/4c1f/a5b3/4fccf0032c26/640x640.jpg","thumbnailWidth":640,"thumbnailHeight":640,"apiProvider":"tidal","platforms":["tidal"]},"YOUTUBE_VIDEO::SZsvRgqi3Fc":{"id":"SZsvRgqi3Fc","type":"song","title":"What We Worked For","artistName":"Against Me! - Topic","thumbnailUrl":"https://i.ytimg.com/vi/SZsvRgqi3Fc/hqdefault.jpg","thumbnailWidth":480,"thumbnailHeight":360,"apiProvider":"youtube","platforms":["youtube","youtubeMusic"]}},"linksByPlatform":{"boomplay":{"country":"US","url":"https://www.boomplay.com/songs/20846327","entityUniqueId":"BOOMPLAY_SONG::20846327"},"deezer":{"country":"US","url":"https://www.deezer.com/track/64497787","entityUniqueId":"DEEZER_SONG::64497787"},"napster":{"country":"US","url":"https://play.napster.com/track/tra.7345970","entityUniqueId":"NAPSTER_SONG::tra.7345970"},"pandora":{"country":"US","url":"https://www.pandora.com/TR:5831794","entityUniqueId":"PANDORA_SONG::TR:5831794"},"spotify":{"country":"US","url":"https://open.spotify.com/track/12Pgnvye9Vn1X5e9fAzBiG","nativeAppUriDesktop":"spotify:track:12Pgnvye9Vn1X5e9fAzBiG","entityUniqueId":"SPOTIFY_SONG::12Pgnvye9Vn1X5e9fAzBiG"},"tidal":{"country":"US","url":"https://listen.tidal.com/track/31448515","entityUniqueId":"TIDAL_SONG::31448515"},"youtube":{"country":"US","url":"https://www.youtube.com/watch?v=SZsvRgqi3Fc","entityUniqueId":"YOUTUBE_VIDEO::SZsvRgqi3Fc"},"youtubeMusic":{"country":"US","url":"https://music.youtube.com/watch?v=SZsvRgqi3Fc","entityUniqueId":"YOUTUBE_VIDEO::SZsvRgqi3Fc"},"appleMusic":{"country":"US","url":"https://geo.music.apple.com/us/album/_/44734006?i=44733632&mt=1&app=music&ls=1&at=1000lHKX&ct=api_http&itscg=30200&itsct=odsl_m","nativeAppUriMobile":"music://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=music&ls=1&at=1000lHKX&ct=api_uri_m&itscg=30200&itsct=odsl_m","nativeAppUriDesktop":"itmss://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=music&ls=1&at=1000lHKX&ct=api_uri_d&itscg=30200&itsct=odsl_m","entityUniqueId":"ITUNES_SONG::44733632"},"itunes":{"country":"US","url":"https://geo.music.apple.com/us/album/_/44734006?i=44733632&mt=1&app=itunes&ls=1&at=1000lHKX&ct=api_http&itscg=30200&itsct=odsl_m","nativeAppUriMobile":"itmss://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=itunes&ls=1&at=1000lHKX&ct=api_uri_m&itscg=30200&itsct=odsl_m","nativeAppUriDesktop":"itmss://itunes.apple.com/us/album/_/44734006?i=44733632&mt=1&app=itunes&ls=1&at=1000lHKX&ct=api_uri_d&itscg=30200&itsct=odsl_m","entityUniqueId":"ITUNES_SONG::44733632"}}}"#;

    #[test]
    fn test_build_lines() {
        let body: SonglinkResponseBody = serde_json::from_str(songlink_output).unwrap();

        let _slack_lines = build_songlink_main_block(&body);
    }

    #[test]
    fn test_build_msg() {
        let body: SonglinkResponseBody = serde_json::from_str(songlink_output).unwrap();

        let slack_msg = build_songlink_slack_message(vec![body]);

        println!("{:?}", serde_json::to_string(&slack_msg).unwrap());
    }

    #[test]
    fn test_build_full_msg() {
        let body: SonglinkResponseBody = serde_json::from_str(songlink_output).unwrap();

        let slack_msg = build_songlink_full_msg(&body);
        write_serde_struct_to_file("testing.json", &slack_msg);

        println!("{:?}", serde_json::to_string(&slack_msg).unwrap());
    }
}
