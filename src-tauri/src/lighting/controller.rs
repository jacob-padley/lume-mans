use std::net::Ipv4Addr;

use crate::lighting::command::{LightingCommand, PlaybackHandle, PlaybackLevel};

pub struct LightingController {
    ip_address: Ipv4Addr,
    port: u32,
    client: reqwest::Client,
}

impl LightingController {
    pub fn new(ip_address: Ipv4Addr, port: u32) -> Self {
        let client = reqwest::Client::new();
        Self {
            ip_address,
            port,
            client,
        }
    }

    pub async fn send(&self, command: LightingCommand) -> anyhow::Result<()> {
        let route = match command {
            LightingCommand::PlayPlayback(command) => {
                let handle_arg = match command.handle {
                    PlaybackHandle::UserNumber(arg) => format!("handle_userNumber={}", arg),
                    PlaybackHandle::Location(arg) => format!("handle_location={}", arg),
                    PlaybackHandle::TitanId(arg) => format!("handle_titanId={}", arg),
                };
                let level_arg = match command.level {
                    PlaybackLevel::Level(arg) => format!("level_level={}", arg),
                    PlaybackLevel::LevelDelta(arg) => format!("level_leveldelta={}", arg),
                };
                format!(
                    "{}/Playbacks/PlayPlayback?handle_{}&{}&accuracy={}",
                    self.api_base(),
                    handle_arg,
                    level_arg,
                    command.accuracy
                )
            }
            LightingCommand::ReleaseAllPlaybacks(command) => format!(
                "{}/Playbacks/ReleaseAllPlaybacks?fadeTime={}&useMasterReleaseTime={}",
                self.api_base(),
                command.fade_time,
                command.use_master_release_time
            ),
        };
        match self.client.get(&route).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    Ok(())
                } else {
                    let reason = response
                        .text()
                        .await
                        .unwrap_or(String::from("Failed to read response body"));
                    Err(anyhow::anyhow!("HTTP error: {}: {}", status, reason))
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    Err(anyhow::anyhow!("Request timed out to URL: {}", &route))
                } else if e.is_connect() {
                    Err(anyhow::anyhow!("Connection failure to URL: {}", &route))
                } else {
                    Err(anyhow::anyhow!("Unknown error: {}", e))
                }
            }
        }
    }

    fn api_base(&self) -> String {
        format!("http://{}:{}/titan/script/2", self.ip_address, self.port)
    }
}
