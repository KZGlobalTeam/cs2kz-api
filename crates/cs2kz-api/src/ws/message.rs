use std::collections::HashMap;
use std::fmt;
use std::net::Ipv4Addr;
use std::time::Duration;

use axum::extract::ws::Message as RawMessage;
use cs2kz::announcements::Announcement;
use cs2kz::checksum::Checksum;
use cs2kz::maps::{CourseFilterId, Map, MapId};
use cs2kz::mode::{Mode, ModeInfo};
use cs2kz::pagination::{Limit, Offset};
use cs2kz::players::{PlayerId, PlayerInfo, PlayerInfoWithIsBanned, Preferences};
use cs2kz::records::{Record, RecordId, StylesForNewRecord, SubmittedPB};
use cs2kz::styles::{StyleInfo, Styles};
use cs2kz::time::Seconds;

use crate::maps::{CourseInfo, MapIdentifier, MapInfo};
use crate::players::PlayerIdentifier;

/// A WebSocket message.
///
/// The generic `T` is used to separate payload types for incoming/outgoing messages.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Message<T> {
    /// An ID set by the client.
    ///
    /// We pass this value along so the client can tie our responses back to their original
    /// requests.
    id: u32,

    /// The payload.
    #[serde(flatten)]
    payload: T,
}

/// The initial payload sent by CS2 servers after connecting.
#[derive(Debug, serde::Deserialize)]
pub struct Hello {
    /// The cs2kz-metamod version the server is currently running.
    pub plugin_version: semver::Version,

    pub plugin_version_checksum: Checksum,

    /// The name of the map the server is currently hosting.
    pub map: String,

    /// Players currently on the server.
    pub players: HashMap<PlayerId, PlayerInfo>,
}

/// The API's response to a [`Hello`] message.
#[derive(Debug, serde::Serialize)]
pub struct HelloAck {
    /// The interval at which the client should send ping messages (in seconds).
    pub heartbeat_interval: Seconds,

    /// Detailed information about the map the server is currently hosting.
    pub map: Option<Map>,

    /// Checksums of all global modes.
    pub modes: Vec<ModeInfo>,

    /// Checksums of all global styles.
    pub styles: Vec<StyleInfo>,

    pub announcements: Vec<Announcement>,
}

/// An error occurred on the side of the API.
#[derive(Debug, serde::Serialize)]
pub struct Error {
    message: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", tag = "event", content = "data")]
pub enum Incoming {
    /// The server changed map.
    MapChange {
        new_map: String,
    },

    /// The server wants information about a map.
    WantMapInfo {
        map: MapIdentifier,
    },

    /// A player joined the server.
    PlayerJoin {
        id: PlayerId,
        name: String,
        ip_address: Ipv4Addr,
    },

    /// A player left the server.
    PlayerLeave {
        id: PlayerId,
        name: String,
        preferences: Preferences,
    },

    /// The server wants a player's preferences.
    WantPreferences {
        player_id: PlayerId,
    },

    /// The server wants all world records for a map.
    WantWorldRecordsForCache {
        map_id: MapId,
    },

    WantCourseTop {
        map_name: String,
        course: String,
        mode: Mode,
        limit: Limit<1000, 100>,
        offset: Offset,
    },

    /// The server wants all PBs of a player for a map.
    WantPlayerRecords {
        map_id: MapId,
        player_id: PlayerId,
    },

    WantPersonalBest {
        player: PlayerIdentifier,
        map: MapIdentifier,
        course: String,
        mode: Mode,

        #[expect(dead_code, reason = "TODO")]
        styles: Styles,
    },

    WantWorldRecords {
        map: MapIdentifier,
        course: String,
        mode: Mode,
    },

    /// A player submitted a record.
    NewRecord {
        player_id: PlayerId,
        filter_id: CourseFilterId,
        mode_md5: Checksum,
        styles: StylesForNewRecord,
        teleports: u32,
        time: Seconds,
    },
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "event", content = "data")]
pub enum Outgoing {
    MapInfo {
        map: Option<Map>,
    },
    PlayerJoinAck {
        is_banned: bool,
        preferences: Preferences,
    },
    PlayerPreferences {
        preferences: Option<Preferences>,
    },
    WorldRecordsForCache {
        records: Vec<Record>,
    },
    CourseTop {
        map: Option<MapInfo>,
        course: Option<CourseInfo>,
        overall: Vec<Record>,
        pro: Vec<Record>,
    },
    PlayerRecords {
        records: Vec<Record>,
    },
    PersonalBest {
        player: Option<PlayerInfoWithIsBanned>,
        map: Option<MapInfo>,
        course: Option<CourseInfo>,
        overall: Option<Record>,
        pro: Option<Record>,
    },
    WorldRecords {
        map: Option<MapInfo>,
        course: Option<CourseInfo>,
        overall: Option<Record>,
        pro: Option<Record>,
    },
    NewRecordAck {
        record_id: RecordId,
        pb_data: Option<SubmittedPB>,
    },
}

#[derive(Debug, Display, Error, From)]
#[display("failed to decode incoming message: {_0}")]
pub struct DecodeMessageError(serde_json::Error);

#[derive(Debug, Display, Error, From)]
#[display("failed to encode outgoing message: {_0}")]
pub struct EncodeMessageError(serde_json::Error);

impl<T> Message<T> {
    pub fn payload(&self) -> &T {
        &self.payload
    }

    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T: for<'de> serde::Deserialize<'de>> Message<T> {
    /// Decodes an incoming message.
    #[tracing::instrument(skip(payload), err(level = "debug"))]
    pub fn decode(payload: &[u8]) -> Result<Self, DecodeMessageError> {
        serde_json::from_slice(payload)
            .inspect_err(|_| debug!(payload = ?String::from_utf8_lossy(payload)))
            .map_err(DecodeMessageError)
    }
}

impl<T: serde::Serialize> Message<T> {
    /// Encodes an outgoing message.
    pub fn encode(&self) -> Result<RawMessage, EncodeMessageError> {
        serde_json::to_string(self)
            .map(|text| RawMessage::Text(text.into()))
            .map_err(EncodeMessageError)
    }
}

impl Message<HelloAck> {
    /// Acknowledges a [`Hello`] message with a [`HelloAck`].
    pub fn ack_hello(
        hello: &Message<Hello>,
        heartbeat_interval: Duration,
        map: Option<Map>,
        modes: Vec<ModeInfo>,
        styles: Vec<StyleInfo>,
        announcements: Vec<Announcement>,
    ) -> Self {
        Self {
            id: hello.id,
            payload: HelloAck {
                heartbeat_interval: heartbeat_interval.into(),
                map,
                modes,
                styles,
                announcements,
            },
        }
    }
}

impl Message<Error> {
    /// Sends an error message to the client.
    pub fn error(error: impl fmt::Display) -> Self {
        Self {
            id: 0,
            payload: Error { message: error.to_string() },
        }
    }
}

impl Message<Outgoing> {
    /// Creates a reply to an incoming message with the given payload.
    pub fn reply(to: &Message<Incoming>, payload: Outgoing) -> Self {
        Self { id: to.id, payload }
    }
}
