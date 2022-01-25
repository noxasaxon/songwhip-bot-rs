# songwhip-bot-rs

Slack bot that queries Songwhip when it detects music URLs.
### Supported Features
- Invite the app to a channel to automatically post threaded Songwhip messages when it detects a music URL
- Or use `/song` command to query Songwhip directly

<img src=./songwhip-bot.png width="300px" >

- [songwhip-bot-rs](#songwhip-bot-rs)
    - [Supported Features](#supported-features)
  - [Local Development](#local-development)
    - [Prerequisites](#prerequisites)
      - [Step 1 - setup .env and start server](#step-1---setup-env-and-start-server)
      - [Step 2 - Start ngrok and connect Slack to it](#step-2---start-ngrok-and-connect-slack-to-it)
  - [Deployment via Fly.io](#deployment-via-flyio)
  - [Creating the Slack App & Permissions, URLs, Slash Commands, etc.](#creating-the-slack-app--permissions-urls-slash-commands-etc)

## Local Development
### Prerequisites
- ngrok

#### Step 1 - setup .env and start server
1. Create a `.env` File
    ```
      SLACK_BOT_TOKEN=<xoxb-1234567>
      SLACK_SIGNING_SECRET=<slack-signing-secret>
    ```

2. `cargo run --bin bot_server --features ansi`


#### Step 2 - Start ngrok and connect Slack to it 
1. In a new Terminal, at the ngrok installation directory: `ngrok http 3000` or `./ngrok http --region=us --hostname=<custom_name>.ngrok.io 3000`
2. Get the https url from ngrok and replace all instances of `<MY_BOT_URL_HERE>` in the `./manifest.yml`
3. Paste the updated `manifest` in your Slack App @ https://api.slack.com/apps - ([Setting up the Slack App](#creating-the-slack-apps-permissions-urls-slash-commands-etc))
4. You may need to install or reinstall the App from the `Basic Information` tab.
5. It may ask you to verify the Event Subscription URL, if your local bot has started and ngrok is running then this verification should succeed.

## Deployment via Fly.io
1. [Install Fly's CLI](https://fly.io/docs/hands-on/installing/)
2. Sign Up or Sign In
   1. `flyctl auth signup`
   2. `flyctl auth login` 
3. Create a Fly app with `flyctl launch` and choose `Y` to copy the existing `fly.toml` file
   1. **hint** the "app name" will form part of the free hostname url that fly provides you if you don't want a custom domain
   2. Don't deploy yet, it will fail because we haven't set up secrets.
4. Provision Secrets
   1. ```flyctl secrets set SLACK_BOT_TOKEN=<xoxb-My_Bot_Token> SLACK_SIGNING_SECRET=<my_signing_secret>```
5. Deploy
   1. `flyctl deploy`
   2. **Bug**: if it fails to deploy/redeploy because it was "killed" during cargo install, try running `flyctl deploy --remote-only` and see if that works
6. Setup Slack
   1.  Copy the host url from `flyctl open` and use it to replace all instances of `<MY_BOT_URL_HERE>` in the `./manifest.yml`
   2. Paste the updated `manifest` in your Slack App @ https://api.slack.com/apps - ([Setting up the Slack App](#creating-the-slack-apps-permissions-urls-slash-commands-etc))
   3. You may need to install or reinstall the App from the `Basic Information` tab.
   4.  It may ask you to verify the Event Subscription URL, if your URL is correct and the Fly app is running, then this verification should succeed.


## Creating the Slack App & Permissions, URLs, Slash Commands, etc.
The bot's Slack configuration is in a single `./manifest.yml` file can be pasted into your Slack App Manifest (either when creating a new app or modifying an existing one). You will just need to replace all instances of `<MY_BOT_URL_HERE>` in the `manifest.yml` with the actual URL of your deployed (or local) application.
