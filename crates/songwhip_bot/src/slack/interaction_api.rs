use super::SlackStateWorkaround;
use axum::{
    extract::{Extension, Form},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, Value};
use slack_morphism::prelude::*;
use std::sync::Arc;
use tracing::error;

/// To `ack` the event, Slack needs empty content or a 204 status code like (StatusCode::OK, "")
pub async fn axum_handler_slack_interactions_api(
    Extension(slack_state): Extension<Arc<SlackStateWorkaround>>,
    Form(body): Form<SlackInteractionWrapper>,
) -> impl IntoResponse {
    let response = handle_slack_interaction(&*slack_state, body).await;
    (response.0, Json(response.1))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackInteractionWrapper {
    // payload: SlackInteractionEvent, // but as a Form
    payload: String,
}

pub async fn handle_slack_interaction(
    _slack_state: &SlackStateWorkaround,
    payload: SlackInteractionWrapper,
) -> (StatusCode, Value) {
    if let Ok(interaction_event) = from_str::<SlackInteractionEvent>(&payload.payload) {
        match interaction_event {
            SlackInteractionEvent::BlockActions(_block_action_event) => todo!(),
            SlackInteractionEvent::ViewSubmission(_view_submission_event) => todo!(),
            SlackInteractionEvent::ViewClosed(..) => todo!(),
            SlackInteractionEvent::DialogSubmission(_) => todo!(),
            SlackInteractionEvent::MessageAction(_) => todo!(),
            SlackInteractionEvent::Shortcut(_) => todo!(),
        }

        // (StatusCode::NO_CONTENT, serde_json::to_value("").unwrap())
    } else {
        error!("Interaction event `payload` key is not valid json or does not deserialize to existing struct");
        error!("{:?}", &payload);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            serde_json::to_value("").unwrap(),
        )
    }
}
