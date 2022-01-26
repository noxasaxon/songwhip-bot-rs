use super::SlackStateWorkaround;
use crate::{
    check_slash_command_for_urls, events_api::build_songwhip_slack_message,
    songwhip::songwhip_query,
};
use axum::{
    body,
    extract::{Extension, Form},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tracing::{debug, error};

/// slash commands?
pub async fn axum_handler_handle_slack_commands_api(
    Extension(slack_state): Extension<Arc<SlackStateWorkaround>>,
    Form(payload): Form<SlackCommandEvent>,
) -> impl IntoResponse {
    handle_slack_command(slack_state, payload).await;

    Response::builder()
        .status(StatusCode::OK)
        .header("X-Slack-No-Retry", "1")
        .body(body::boxed(body::Full::new(String::default().into())))
        .unwrap()
}

// separate into a non-axum function for possible use without Axum (e.g. Lambda function)
pub async fn handle_slack_command(
    slack_state: Arc<SlackStateWorkaround>,
    payload: SlackCommandEvent,
) {
    if let Some(message) = payload.text {
        let msg_urls = check_slash_command_for_urls(&message);

        if msg_urls.is_empty() {
            debug!("No urls found in slash command");
            return;
        }

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

            if !valid_results.is_empty() {
                let session = slack_state.open_session();
                if let Ok(convo_open) = session
                    .conversations_open(
                        &SlackApiConversationsOpenRequest::new().with_users(vec![payload.user_id]),
                    )
                    .await
                {
                    if let Err(slack_err) = session
                        .chat_post_message(
                            &SlackApiChatPostMessageRequest::new(
                                convo_open.channel.id,
                                build_songwhip_slack_message(valid_results),
                            )
                            .opt_unfurl_links(Some(false))
                            .opt_unfurl_media(Some(false)),
                        )
                        .await
                    {
                        error!("Failed to DM user: {}", slack_err);
                    }
                }
            }
        });
    }
}
