use std::collections::HashMap;

use anyhow::{Result, bail};
use futures_util::StreamExt;
use veila_common::NowPlayingConfig;
use zbus::{Connection, MatchRule, MessageStream, message::Type, zvariant::OwnedValue};

use super::{
    DBUS_INTERFACE, DBUS_PROPERTIES_INTERFACE, MPRIS_INTERFACE, MPRIS_NAMESPACE, MPRIS_PATH,
    NowPlayingRefresh, fetch_snapshot,
};

pub(super) struct MprisClient {
    connection: Connection,
    name_changes: MessageStream,
    property_changes: MessageStream,
}

impl MprisClient {
    pub(super) async fn connect() -> Result<Self> {
        let connection = Connection::session().await?;
        let name_changes = mpris_name_owner_changed_stream(&connection).await?;
        let property_changes = mpris_properties_changed_stream(&connection).await?;

        Ok(Self {
            connection,
            name_changes,
            property_changes,
        })
    }

    pub(super) async fn refresh(&self, config: &NowPlayingConfig) -> Result<NowPlayingRefresh> {
        fetch_snapshot(&self.connection, config).await
    }

    pub(super) async fn wait_for_change(&mut self) -> Result<MprisWakeReason> {
        loop {
            tokio::select! {
                message = self.name_changes.next() => {
                    stream_message(message, "mpris name-owner signal")?;
                    return Ok(MprisWakeReason::NameOwnerChanged);
                }
                message = self.property_changes.next() => {
                    let message = stream_message(message, "mpris properties signal")?;
                    if properties_changed_affects_snapshot(&message) {
                        return Ok(MprisWakeReason::PropertiesChanged);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MprisWakeReason {
    NameOwnerChanged,
    PropertiesChanged,
}

fn stream_message(
    message: Option<zbus::Result<zbus::Message>>,
    label: &str,
) -> Result<zbus::Message> {
    match message {
        Some(Ok(message)) => Ok(message),
        Some(Err(error)) => Err(error.into()),
        None => {
            bail!("{label} stream ended");
        }
    }
}

fn properties_changed_affects_snapshot(message: &zbus::Message) -> bool {
    let Ok((interface, changed, invalidated)) =
        message
            .body()
            .deserialize::<(String, HashMap<String, OwnedValue>, Vec<String>)>()
    else {
        return true;
    };

    interface == MPRIS_INTERFACE
        && (changed.contains_key("Metadata")
            || changed.contains_key("PlaybackStatus")
            || invalidated
                .iter()
                .any(|property| property == "Metadata" || property == "PlaybackStatus"))
}

async fn mpris_name_owner_changed_stream(connection: &Connection) -> Result<MessageStream> {
    let rule = mpris_name_owner_changed_rule()?;

    MessageStream::for_match_rule(rule, connection, Some(16))
        .await
        .map_err(Into::into)
}

fn mpris_name_owner_changed_rule() -> Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(DBUS_INTERFACE)?
        .interface(DBUS_INTERFACE)?
        .member("NameOwnerChanged")?
        .arg0ns(MPRIS_NAMESPACE)?
        .build())
}

async fn mpris_properties_changed_stream(connection: &Connection) -> Result<MessageStream> {
    let rule = mpris_properties_changed_rule()?;
    MessageStream::for_match_rule(rule, connection, Some(32))
        .await
        .map_err(Into::into)
}

fn mpris_properties_changed_rule() -> Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(DBUS_PROPERTIES_INTERFACE)?
        .member("PropertiesChanged")?
        .path(MPRIS_PATH)?
        .add_arg(MPRIS_INTERFACE)?
        .build())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        mpris_name_owner_changed_rule, mpris_properties_changed_rule,
        properties_changed_affects_snapshot,
    };
    use zbus::{Message, zvariant::Value};

    #[test]
    fn mpris_name_owner_rule_tracks_player_namespace() {
        let rule = mpris_name_owner_changed_rule().expect("match rule");
        let rule = rule.to_string();

        assert!(rule.contains("member='NameOwnerChanged'"));
        assert!(rule.contains("arg0namespace='org.mpris.MediaPlayer2'"));
    }

    #[test]
    fn mpris_properties_rule_tracks_player_metadata_changes() {
        let rule = mpris_properties_changed_rule().expect("match rule");
        let rule = rule.to_string();

        assert!(rule.contains("interface='org.freedesktop.DBus.Properties'"));
        assert!(rule.contains("member='PropertiesChanged'"));
        assert!(rule.contains("path='/org/mpris/MediaPlayer2'"));
        assert!(rule.contains("arg0='org.mpris.MediaPlayer2.Player'"));
    }

    #[test]
    fn properties_changed_filter_accepts_snapshot_fields() {
        let message = properties_changed_message(&[("Metadata", Value::from("track"))], &[]);

        assert!(properties_changed_affects_snapshot(&message));
    }

    #[test]
    fn properties_changed_filter_ignores_unrelated_fields() {
        let message = properties_changed_message(&[("Position", Value::from(42_i64))], &[]);

        assert!(!properties_changed_affects_snapshot(&message));
    }

    fn properties_changed_message(changed: &[(&str, Value<'_>)], invalidated: &[&str]) -> Message {
        let changed = changed.iter().cloned().collect::<HashMap<_, _>>();
        Message::signal(
            super::MPRIS_PATH,
            super::DBUS_PROPERTIES_INTERFACE,
            "PropertiesChanged",
        )
        .expect("signal")
        .build(&(super::MPRIS_INTERFACE, changed, invalidated))
        .expect("message")
    }
}
