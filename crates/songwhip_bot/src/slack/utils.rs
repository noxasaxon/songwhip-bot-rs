use serde::{Deserialize, Serialize};
use slack_morphism::prelude::*;
use slack_morphism_hyper::{
    SlackClientHyperConnector, SlackClientHyperHttpsConnector, SlackHyperClient,
};
use std::{collections::HashMap, env, sync::Arc};
use tracing::error;
use url::{ParseError, Url};

/// Helper for slack token->client persistence
pub struct SlackStateWorkaround {
    slack_client: SlackHyperClient,
    bot_token: SlackApiToken,
}

impl SlackStateWorkaround {
    pub fn new(bot_token: &str) -> Self {
        Self {
            bot_token: SlackApiToken::new(bot_token.into()),
            slack_client: SlackClient::new(SlackClientHyperConnector::new()),
        }
    }

    pub fn new_from_env() -> Self {
        SlackStateWorkaround {
            bot_token: SlackApiToken::new(
                std::env::var("SLACK_BOT_TOKEN")
                    .unwrap_or_else(|_| "<no_token_provided>".to_string())
                    .into(),
            ),
            slack_client: SlackClient::new(SlackClientHyperConnector::new()),
        }
    }

    pub fn open_session(&self) -> SlackClientSession<SlackClientHyperHttpsConnector> {
        self.slack_client.open_session(&self.bot_token)
    }
}

pub fn setup_slack() -> Arc<SlackStateWorkaround> {
    // SETUP SHARED SLACK CLIENT
    let slack_bot_token = SlackApiToken::new(
        env::var("SLACK_BOT_TOKEN")
            .unwrap_or_else(|_| "<no_token_provided".to_string())
            .into(),
    );
    let slack_client = SlackClient::new(SlackClientHyperConnector::new());

    Arc::new(SlackStateWorkaround {
        bot_token: slack_bot_token,
        slack_client,
    })
}

static RE: tokio::sync::OnceCell<regex::Regex> = tokio::sync::OnceCell::const_new();

/// Slack's regex for when it is in a Message (formatted url)
async fn url_regex() -> regex::Regex {
    regex::Regex::new(r#"<(?P<url>.*?)[\||>]"#).unwrap()
}

pub async fn check_slack_formatted_message_for_urls(message: &str) -> Vec<String> {
    let re = RE.get_or_init(url_regex).await;
    re.captures_iter(message)
        .map(|caps| caps["url"].to_string())
        .collect()
}

pub fn check_slash_command_for_urls(raw_text: &str) -> Vec<String> {
    let words = raw_text.split_whitespace();
    let mut urls = Vec::default();

    for word in words {
        if let Err(parse_err) = Url::parse(word) {
            if parse_err == ParseError::RelativeUrlWithoutBase {
                let with_base = format!("https://{}", word);
                if let Ok(_correct_url) = Url::parse(&with_base) {
                    urls.push(with_base)
                }
            }
        } else {
            urls.push(word.into());
        }
    }

    urls
}

/// Attributes to describe an incoming message event
pub trait MessageHelpers {
    fn is_bot_message(&self) -> bool {
        false
    }

    fn is_threaded(&self) -> bool {
        false
    }

    fn is_hidden(&self) -> bool {
        false
    }
}

impl MessageHelpers for SlackMessageEvent {
    fn is_bot_message(&self) -> bool {
        matches!(self.subtype, Some(SlackMessageEventType::BotMessage))
            || self.sender.bot_id.is_some()
    }

    fn is_threaded(&self) -> bool {
        self.origin.thread_ts.is_some()
    }

    fn is_hidden(&self) -> bool {
        self.hidden.is_some()
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "response_action", rename_all = "snake_case")]
pub enum SlackResponseAction {
    /// HashMap<SlackBlockId -> error_message>
    Errors { errors: HashMap<String, String> },
    // Update,
}

impl SlackResponseAction {
    pub fn from_validation_errors(errors: Vec<SlackBlockValidationError>) -> Self {
        let mut error_map: HashMap<String, String> = HashMap::new();

        for e in errors {
            error_map.insert(e.block_id.to_string(), e.error_message);
        }

        SlackResponseAction::Errors { errors: error_map }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SlackBlockValidationError {
    pub block_id: SlackBlockId,
    pub error_message: String,
}

pub async fn is_bot_in_channel(
    slack_state: &SlackStateWorkaround,
    channel_id: SlackChannelId,
) -> bool {
    // check if bot is allowed in the channel
    match slack_state
        .open_session()
        .conversations_info(&SlackApiConversationsInfoRequest::new(channel_id))
        .await
    {
        Ok(resp) => match resp.channel.flags.is_member {
            Some(membership) => membership,
            None => {
                error!("no membership present for this channel: {:?}", resp);
                false
            }
        },
        Err(slack_error) => {
            error!(
                "Failed to fetch conversation info when checking if bot in channel: {}",
                slack_error
            );
            false
        }
    }
}

pub fn add_emoji_colons(emoji_name: &str) -> String {
    match emoji_name.as_bytes() {
        [b':', .., b':'] => emoji_name.to_string(),
        [b':', ..] => format!("{emoji_name}:"),
        [.., b':'] => format!(":{emoji_name}"),
        [..] => format!(":{emoji_name}:"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_emoji_colons() {
        assert_eq!(":rust:", add_emoji_colons(":rust:"));
        assert_eq!(":rust:", add_emoji_colons(":rust"));
        assert_eq!(":rust:", add_emoji_colons("rust:"));
        assert_eq!(":rust:", add_emoji_colons("rust"));
    }
}
