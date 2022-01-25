use super::SlackStateWorkaround;
use crate::{
    check_slack_formatted_message_for_urls,
    songwhip::{songwhip_query, SongwhipResponseBody},
    MessageHelpers,
};
use axum::{
    body::{self},
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tracing::{debug, error};

pub async fn axum_handler_slack_events_api(
    Extension(slack_state): Extension<Arc<SlackStateWorkaround>>,
    // Json(payload): Json<SlackPushEvent>,
    Json(payload): Json<Value>, // need to do add variant upstream to slack-morphism library
) -> impl IntoResponse {
    let (status, body) = match from_value::<SlackPushEvent>(payload.clone()) {
        Ok(push_event) => handle_slack_event(slack_state, push_event).await,
        Err(_) => {
            let link_shared_payload: EventWrapper = from_value(payload).unwrap();

            let msg_urls: Vec<String> = link_shared_payload
                .event
                .links
                .into_iter()
                .map(|link_obj| link_obj.url)
                .collect();

            process_urls_and_post_songwhip_message(
                msg_urls,
                slack_state,
                link_shared_payload.event.channel,
                link_shared_payload.event.message_ts,
            );

            (StatusCode::OK, Value::default())
        }
    };

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
            let response_body =
                process_event_callback_for_songwhip_bot(event_req, slack_state).await;
            (StatusCode::OK, response_body)
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

pub async fn process_event_callback_for_songwhip_bot(
    event_req: SlackPushEventCallback,
    slack_client: Arc<SlackStateWorkaround>,
    // slack_client: &SlackStateWorkaround,
) -> Value {
    let default_event_response = Value::default();
    match event_req.event {
        SlackEventCallbackBody::Message(event) => {
            if [
                event.is_bot_message(),
                event.is_hidden(),
                event.is_threaded(),
            ]
            .iter()
            .any(|x| !!x)
            {
                println!("IGNORED");
                return default_event_response;
            }

            let message_content = event.content.unwrap().text.unwrap();

            let event_channel_id = event
                .origin
                .channel
                .unwrap_or_else(|| SlackChannelId("".to_string()));

            let msg_urls = check_slack_formatted_message_for_urls(&message_content).await;
            if !msg_urls.is_empty() {
                process_urls_and_post_songwhip_message(
                    msg_urls,
                    slack_client,
                    event_channel_id,
                    event.origin.ts,
                );
            }
        }
        SlackEventCallbackBody::AppHomeOpened(_event) => todo!(),
        SlackEventCallbackBody::AppMention(_event) => todo!(),
        SlackEventCallbackBody::AppUninstalled(_event) => todo!(),
    }

    default_event_response
}

pub fn build_songwhip_slack_message(
    sw_responses: Vec<SongwhipResponseBody>,
) -> SlackMessageContent {
    let song_sections: Vec<SlackBlock> = sw_responses
        .iter()
        .map(|sw_resp| build_songwhip_line_blocks(sw_resp.clone()).into())
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

#[derive(Serialize, Deserialize)]
pub struct EventWrapper {
    event: LinkSharedEvent,
}

#[derive(Serialize, Deserialize)]
/// Need to push this back upstream to slack-morphism library
pub struct LinkSharedEvent {
    channel: SlackChannelId,
    event_ts: SlackTs,
    is_bot_user_member: bool,
    links: Vec<SlackLinkObject>,
    message_ts: SlackTs,
    source: String,
    // "type": "link_shared",
    // "unfurl_id": "C02V85P7D0T.1643077335.005400.9b54297220747181404235da101b11ab86e421c3d7440818e04ba80406c06ee0",
    user: SlackUserId,
}

#[derive(Serialize, Deserialize)]
pub struct SlackLinkObject {
    domain: String,
    url: String,
}
