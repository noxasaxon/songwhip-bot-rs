use super::SlackStateWorkaround;
use crate::songwhip::{songwhip_query, SongwhipResponseBody};
use axum::{
    body::{self},
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
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

                        process_urls_and_post_songwhip_message(
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

pub fn process_urls_and_post_songwhip_message(
    msg_urls: Vec<String>,
    slack_state: Arc<SlackStateWorkaround>,
    slack_channel_id: SlackChannelId,
    msg_timestamp: SlackTs,
) {
    tokio::spawn(async move {
        let mut valid_results = Vec::default();

        for msg_url in msg_urls {
            match songwhip_query(&msg_url).await {
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
                    build_songwhip_slack_message(valid_results),
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

pub fn build_songwhip_slack_message(
    sw_responses: Vec<SongwhipResponseBody>,
) -> SlackMessageContent {
    let song_sections: Vec<SlackBlock> = sw_responses
        .iter()
        .map(|sw_resp| build_songwhip_line_blocks(sw_resp).into())
        .collect();

    SlackMessageContent::new().with_blocks(song_sections)
}

pub fn build_songwhip_line_blocks(sw_resp: &SongwhipResponseBody) -> SlackSectionBlock {
    let artists_names: String = sw_resp
        .artists
        .iter()
        .map(|artist| artist.name.clone())
        .reduce(|accum, item| format!("{accum}, {item}"))
        .unwrap();

    let section = SlackSectionBlock::new().with_text(md!(format!(
        "<{}|_*{}*_> \n by {}",
        sw_resp.url.clone(),
        sw_resp.name,
        artists_names
    )));
    if let Some(image_url) = sw_resp.image.clone() {
        section.with_accessory(SlackSectionBlockElement::Image(
            SlackBlockImageElement::new(image_url, "songwhip song image".into()),
        ))
    } else {
        section
    }
}
