use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::io;
use std::ops::ControlFlow;
use std::pin::pin;
use std::time::Duration;

use axum::extract::ws::{CloseFrame, Message as RawMessage, close_code};
use cs2kz::Context;
use cs2kz::pagination::{Limit, Offset};
use cs2kz::players::{NewPlayer, PlayerId, PlayerInfo, PlayerInfoWithIsBanned};
use cs2kz::plugin::PluginVersionId;
use cs2kz::records::{GetRecordsParams, NewRecord};
use cs2kz::servers::ServerId;
use futures_util::{Sink, SinkExt, Stream, TryStreamExt};
use tokio::time::{MissedTickBehavior, interval, sleep};
use tokio_util::sync::CancellationToken;

use self::message::Message;
use crate::maps::{CourseInfo, MapIdentifier, MapInfo};
use crate::players::PlayerIdentifier;
use crate::runtime;

pub mod message;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
const DEBOUNCE: Duration = Duration::from_millis(100);

struct State {
    server_id: ServerId,
    plugin_version_id: PluginVersionId,
    players: HashMap<PlayerId, PlayerInfo>,
}

/// Handles a WebSocket connection from a CS2 server.
///
/// CS2 servers are expected to send a "hello" message as their first message
/// (see [`perform_handshake`]).
///
/// Afterwards, they can send messages in the shape of [`message::Incoming`] as they please.
///
/// They also need to send pings at a [fixed interval](HEARTBEAT_INTERVAL), or we will close the
/// connection.
#[tracing::instrument(skip_all, err)]
pub async fn handle_connection<C, E>(
    cx: Context,
    shutdown_token: CancellationToken,
    server_id: ServerId,
    mut conn: C,
) -> io::Result<()>
where
    C: Stream<Item = Result<RawMessage, E>> + Sink<RawMessage, Error: Into<BoxError>> + Unpin,
    E: Into<BoxError>,
{
    let ControlFlow::Continue(mut state) =
        perform_handshake(&cx, &shutdown_token, &mut conn, server_id)
            .await
            .map_err(io::Error::other)?
    else {
        return conn
            .send(RawMessage::Close(Some(unauthorized_close_frame())))
            .await
            .map_err(io::Error::other);
    };

    let mut heartbeat_interval = interval(HEARTBEAT_INTERVAL);
    heartbeat_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    heartbeat_interval.tick().await;

    let mut debounce = interval(DEBOUNCE);
    debounce.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        debounce.tick().await;

        let message = select! {
            () = shutdown_token.cancelled() => {
                debug!("server shutting down; closing connection");

                conn.send(RawMessage::Close(Some(shutdown_close_frame())))
                    .await
                    .map_err(io::Error::other)?;

                break Ok(());
            },

            _ = heartbeat_interval.tick() => {
                debug!("client exceeded heartbeat timeout; closing connection");

                conn.send(RawMessage::Close(Some(timeout_close_frame())))
                    .await
                    .map_err(io::Error::other)?;

                break Ok(());
            },

            message = conn.try_next() => match message.map_err(io::Error::other)? {
                Some(message) => message,
                None => {
                    debug!("client closed the connection");
                    break Ok(());
                },
            },
        };

        let bytes = match message {
            RawMessage::Text(text) => text.into(),
            RawMessage::Binary(bytes) => bytes,
            RawMessage::Ping(_) => {
                trace!("received ping");
                heartbeat_interval.reset();
                continue;
            },
            RawMessage::Pong(_) => {
                trace!("received pong (?)");
                continue;
            },
            RawMessage::Close(close_frame) => {
                debug!(?close_frame, "client closed the connection");
                break Ok(());
            },
        };

        let message = match Message::<message::Incoming>::decode(&bytes) {
            Ok(message) => message,
            Err(error) => {
                debug!(%error, "failed to decode incoming message");

                let reply = Message::error(error).encode().map_err(io::Error::other)?;
                conn.send(reply).await.map_err(io::Error::other)?;
                continue;
            },
        };

        if let Err(error) = handle_message(&cx, &mut conn, &mut state, message).await {
            debug!(%error, "failed to handle message");

            let reply = Message::error(&*error).encode().map_err(io::Error::other)?;
            conn.send(reply).await.map_err(io::Error::other)?;
        }
    }
}

/// Performs the initial handshake.
///
/// Every connection must complete this handshake before doing anything else.
/// Clients are expected to send a "hello" message immediately after connecting, and if they fail
/// to send this message within a timeout, the connection will be closed. The [`ControlFlow`]
/// returned by this function indicates whether the handshake succeeded.
#[tracing::instrument(skip_all, err)]
async fn perform_handshake<C, E>(
    cx: &Context,
    shutdown_token: &CancellationToken,
    conn: &mut C,
    server_id: ServerId,
) -> Result<ControlFlow<(), State>, BoxError>
where
    C: Stream<Item = Result<RawMessage, E>> + Sink<RawMessage, Error: Into<BoxError>> + Unpin,
    E: Into<BoxError>,
{
    let mut timeout = pin!(sleep(HEARTBEAT_INTERVAL));

    loop {
        trace!("waiting for hello message");

        let message = select! {
            () = shutdown_token.cancelled() => {
                debug!("server shutting down; closing connection");

                conn.send(RawMessage::Close(Some(shutdown_close_frame())))
                    .await
                    .map_err(io::Error::other)?;

                break Ok(ControlFlow::Break(()));
            },

            () = &mut timeout => {
                debug!("client exceeded timeout; closing connection");

                conn.send(RawMessage::Close(Some(timeout_close_frame())))
                    .await
                    .map_err(Into::into)?;

                break Ok(ControlFlow::Break(()));
            },

            message = conn.try_next() => match message.map_err(Into::into)? {
                Some(message) => message,
                None => {
                    debug!("client closed the connection");
                    break Ok(ControlFlow::Break(()))
                },
            },
        };

        let bytes = match message {
            RawMessage::Text(text) => text.into(),
            RawMessage::Binary(bytes) => bytes,
            RawMessage::Ping(_) => {
                trace!("received ping, trying again");
                continue;
            },
            RawMessage::Pong(_) => {
                trace!("received pong (?), trying again");
                continue;
            },
            RawMessage::Close(close_frame) => {
                debug!(?close_frame, "client closed the connection");
                break Ok(ControlFlow::Break(()));
            },
        };

        let hello = match Message::<message::Hello>::decode(&bytes) {
            Ok(message) => message,
            Err(error) => {
                debug!(%error, "failed to decode `Hello`");

                let reply = Message::error(error).encode().map_err(io::Error::other)?;
                conn.send(reply).await.map_err(io::Error::other)?;
                break Ok(ControlFlow::Break(()));
            },
        };

        debug!("received `Hello`, validating plugin version");

        let Some(plugin_version) =
            cs2kz::plugin::get_version(cx, &hello.payload().plugin_version).await?
        else {
            debug!(plugin_version = %hello.payload().plugin_version, "unknown plugin version");

            let reply = Message::error("unknown plugin version")
                .encode()
                .map_err(io::Error::other)?;

            conn.send(reply).await.map_err(io::Error::other)?;
            break Ok(ControlFlow::Break(()));
        };

        if !runtime::environment().is_local()
            && !cs2kz::plugin::is_valid_version(
                cx,
                plugin_version.id,
                &hello.payload().plugin_version_checksum,
            )
            .await?
        {
            debug!(plugin_version = %hello.payload().plugin_version, "invalid plugin version");

            let reply = Message::error("invalid plugin version")
                .encode()
                .map_err(io::Error::other)?;

            conn.send(reply).await.map_err(io::Error::other)?;
            break Ok(ControlFlow::Break(()));
        }

        debug!(map = hello.payload().map, "valid plugin version, getting map details");

        let map = cs2kz::maps::get_by_name(cx, &hello.payload().map)
            .try_next()
            .await?;

        let reply = Message::ack_hello(&hello, HEARTBEAT_INTERVAL, map)
            .encode()
            .map_err(io::Error::other)?;

        conn.send(reply).await.map_err(io::Error::other)?;

        debug!("handshake completed");

        break Ok(ControlFlow::Continue(State {
            server_id,
            plugin_version_id: plugin_version.id,
            players: hello.into_payload().players,
        }));
    }
}

/// Handles a single message.
async fn handle_message<C>(
    cx: &Context,
    conn: &mut C,
    state: &mut State,
    message: Message<message::Incoming>,
) -> Result<(), BoxError>
where
    C: Sink<RawMessage, Error: Into<BoxError>> + Unpin,
{
    use message::Incoming as P;

    match *message.payload() {
        P::MapChange { ref new_map } => {
            trace!("server changed map to '{new_map}'");

            let map = cs2kz::maps::get_by_name(cx, new_map).try_next().await?;
            let reply = Message::reply(&message, message::Outgoing::MapInfo { map }).encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::WantMapInfo { ref map } => {
            let map = match *map {
                MapIdentifier::Id(id) => cs2kz::maps::get_by_id(cx, id).await,
                MapIdentifier::Name(ref name) => {
                    cs2kz::maps::get_by_name(cx, name).try_next().await
                },
            }?;

            let reply = Message::reply(&message, message::Outgoing::MapInfo { map }).encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::PlayerJoin { id, ref name, ip_address } => {
            if let Some(player) = state
                .players
                .insert(id, PlayerInfo { id, name: name.clone() })
            {
                warn!(%player.id, player.name, "double join");
            }

            trace!("{name} joined the server");

            let player_info = cs2kz::players::register(cx, NewPlayer {
                id,
                name: Cow::Borrowed(name),
                ip_address: Some(ip_address),
            })
            .await?;

            let reply = Message::reply(&message, message::Outgoing::PlayerJoinAck {
                is_banned: player_info.is_banned,
                preferences: player_info.preferences,
            })
            .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::PlayerLeave { id, ref name, ref preferences } => {
            if let Some(player) = state.players.remove(&id) {
                trace!("{} ({}) left the server as {}", player.name, player.id, name);
            } else {
                warn!(%id, "previously unknown player left the server as {}", name);
            }

            if !cs2kz::players::on_leave(cx, id, name, preferences).await? {
                warn!(%id, "updated non-existent player?");
            }

            Ok(())
        },

        P::WantPreferences { player_id } => {
            let preferences = cs2kz::players::get_preferences(cx, player_id).await?;
            let reply =
                Message::reply(&message, message::Outgoing::PlayerPreferences { preferences })
                    .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::WantWorldRecordsForCache { map_id } => {
            let params = GetRecordsParams {
                top: true,
                map_id: Some(map_id),
                ..Default::default()
            };

            let records = cs2kz::records::get(cx, params).await?.into_inner();

            let reply =
                Message::reply(&message, message::Outgoing::WorldRecordsForCache { records })
                    .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::WantCourseTop { ref map_name, ref course, mode, limit, offset } => {
            let Some(map_info) = cs2kz::maps::get_by_name(cx, map_name)
                .try_next()
                .await?
                .map(|map| MapInfo { id: map.id, name: map.name })
            else {
                let reply = Message::reply(&message, message::Outgoing::CourseTop {
                    map: None,
                    course: None,
                    overall: Vec::new(),
                    pro: Vec::new(),
                })
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            };

            let Some(course_info) =
                cs2kz::maps::get_course_info_by_name(cx, &map_info.name, course, mode)
                    .await?
                    .map(CourseInfo::from)
            else {
                let reply = Message::reply(&message, message::Outgoing::CourseTop {
                    map: None,
                    course: None,
                    overall: Vec::new(),
                    pro: Vec::new(),
                })
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            };

            let overall = cs2kz::records::get(cx, GetRecordsParams {
                top: true,
                player_id: None,
                server_id: None,
                map_id: None,
                course_id: Some(course_info.id),
                mode: Some(mode),
                has_teleports: None,
                max_rank: None,
                sort_by: cs2kz::records::SortBy::Time,
                sort_order: Some(cs2kz::records::SortOrder::Ascending),
                limit,
                offset,
            })
            .await?;

            let pro = cs2kz::records::get(cx, GetRecordsParams {
                top: true,
                player_id: None,
                server_id: None,
                map_id: None,
                course_id: Some(course_info.id),
                mode: Some(mode),
                has_teleports: Some(false),
                max_rank: None,
                sort_by: cs2kz::records::SortBy::Time,
                sort_order: Some(cs2kz::records::SortOrder::Ascending),
                limit,
                offset,
            })
            .await?;

            let reply = Message::reply(&message, message::Outgoing::CourseTop {
                map: Some(map_info),
                course: Some(course_info),
                overall: overall.into_inner(),
                pro: pro.into_inner(),
            })
            .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::WantPlayerRecords { map_id, player_id } => {
            let records = cs2kz::records::get_player_records(cx, player_id, map_id)
                .try_collect::<Vec<_>>()
                .await?;

            let reply =
                Message::reply(&message, message::Outgoing::PlayerRecords { records }).encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        // TODO: styles
        P::WantPersonalBest {
            ref player,
            ref map,
            ref course,
            mode,
            styles: _,
        } => {
            let player = match *player {
                PlayerIdentifier::Id(id) => cs2kz::players::get_by_id(cx, id).await?,
                PlayerIdentifier::Name(ref name) => cs2kz::players::get_by_name(cx, name).await?,
            }
            .map(|player| PlayerInfoWithIsBanned {
                id: player.id,
                name: player.name,
                is_banned: player.is_banned,
            });

            let Some(map_info) = (match *map {
                MapIdentifier::Id(id) => cs2kz::maps::get_by_id(cx, id)
                    .await?
                    .map(|map| MapInfo { id: map.id, name: map.name }),
                MapIdentifier::Name(ref name) => cs2kz::maps::get_by_name(cx, name)
                    .try_next()
                    .await?
                    .map(|map| MapInfo { id: map.id, name: map.name }),
            }) else {
                let reply = Message::reply(&message, message::Outgoing::PersonalBest {
                    player: None,
                    map: None,
                    course: None,
                    overall: None,
                    pro: None,
                })
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            };

            let Some(course_info) =
                cs2kz::maps::get_course_info_by_name(cx, &map_info.name, course, mode)
                    .await?
                    .map(CourseInfo::from)
            else {
                let reply = Message::reply(&message, message::Outgoing::PersonalBest {
                    player: None,
                    map: None,
                    course: None,
                    overall: None,
                    pro: None,
                })
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            };

            let overall = match player.as_ref() {
                None => None,
                Some(&PlayerInfoWithIsBanned { id, .. }) => {
                    cs2kz::records::get(cx, GetRecordsParams {
                        top: true,
                        player_id: Some(id),
                        server_id: None,
                        map_id: Some(map_info.id),
                        course_id: Some(course_info.id),
                        mode: Some(mode),
                        has_teleports: None,
                        max_rank: None,
                        sort_by: cs2kz::records::SortBy::Time,
                        sort_order: Some(cs2kz::records::SortOrder::Ascending),
                        limit: Limit::new(1),
                        offset: Offset::default(),
                    })
                    .await?
                    .into_inner()
                    .into_iter()
                    .next()
                },
            };

            let pro = match player.as_ref() {
                None => None,
                Some(&PlayerInfoWithIsBanned { id, .. }) => {
                    cs2kz::records::get(cx, GetRecordsParams {
                        top: true,
                        player_id: Some(id),
                        server_id: None,
                        map_id: Some(map_info.id),
                        course_id: Some(course_info.id),
                        mode: Some(mode),
                        has_teleports: Some(false),
                        max_rank: None,
                        sort_by: cs2kz::records::SortBy::Time,
                        sort_order: Some(cs2kz::records::SortOrder::Ascending),
                        limit: Limit::new(1),
                        offset: Offset::default(),
                    })
                    .await?
                    .into_inner()
                    .into_iter()
                    .next()
                },
            };

            let reply = Message::reply(&message, message::Outgoing::PersonalBest {
                player,
                map: Some(map_info),
                course: Some(course_info),
                overall,
                pro,
            })
            .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::WantWorldRecords { ref map, ref course, mode } => {
            let Some(map_info) = (match *map {
                MapIdentifier::Id(id) => cs2kz::maps::get_by_id(cx, id)
                    .await?
                    .map(|map| MapInfo { id: map.id, name: map.name }),
                MapIdentifier::Name(ref name) => cs2kz::maps::get_by_name(cx, name)
                    .try_next()
                    .await?
                    .map(|map| MapInfo { id: map.id, name: map.name }),
            }) else {
                let reply = Message::reply(&message, message::Outgoing::WorldRecords {
                    map: None,
                    course: None,
                    overall: None,
                    pro: None,
                })
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            };

            let Some(course_info) =
                cs2kz::maps::get_course_info_by_name(cx, &map_info.name, course, mode)
                    .await?
                    .map(CourseInfo::from)
            else {
                let reply = Message::reply(&message, message::Outgoing::WorldRecords {
                    map: None,
                    course: None,
                    overall: None,
                    pro: None,
                })
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            };

            let overall = cs2kz::records::get(cx, GetRecordsParams {
                top: true,
                player_id: None,
                server_id: None,
                map_id: Some(map_info.id),
                course_id: Some(course_info.id),
                mode: Some(mode),
                has_teleports: None,
                max_rank: None,
                sort_by: cs2kz::records::SortBy::Time,
                sort_order: Some(cs2kz::records::SortOrder::Ascending),
                limit: Limit::new(1),
                offset: Offset::default(),
            })
            .await?
            .into_inner()
            .into_iter()
            .next();

            let pro = cs2kz::records::get(cx, GetRecordsParams {
                top: true,
                player_id: None,
                server_id: None,
                map_id: Some(map_info.id),
                course_id: Some(course_info.id),
                mode: Some(mode),
                has_teleports: Some(false),
                max_rank: None,
                sort_by: cs2kz::records::SortBy::Time,
                sort_order: Some(cs2kz::records::SortOrder::Ascending),
                limit: Limit::new(1),
                offset: Offset::default(),
            })
            .await?
            .into_inner()
            .into_iter()
            .next();

            let reply = Message::reply(&message, message::Outgoing::WorldRecords {
                map: Some(map_info),
                course: Some(course_info),
                overall,
                pro,
            })
            .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },

        P::NewRecord {
            player_id,
            filter_id,
            ref mode_md5,
            ref styles,
            teleports,
            time,
        } => {
            if !runtime::environment().is_local()
                && !cs2kz::mode::verify_checksum(cx, mode_md5, state.plugin_version_id).await?
            {
                let reply = Message::error("invalid mode checksum").encode()?;

                return conn.send(reply).await.map_err(Into::into);
            }

            let valid_styles = cs2kz::styles::get_for_plugin_version(cx, state.plugin_version_id)
                .try_fold(HashSet::new(), async |mut styles, info| {
                    styles.insert(info.linux_checksum);
                    styles.insert(info.windows_checksum);
                    Ok(styles)
                })
                .await?;

            if !runtime::environment().is_local()
                && let Some(invalid_style) = styles
                    .known_styles
                    .iter()
                    .find(|style| !valid_styles.contains(&style.checksum))
            {
                let reply = Message::error(format_args!(
                    "invalid style checksum for '{}'",
                    invalid_style.style,
                ))
                .encode()?;

                return conn.send(reply).await.map_err(Into::into);
            }

            let record = cs2kz::records::submit(cx, NewRecord {
                player_id,
                server_id: state.server_id,
                filter_id,
                styles: styles.clone(),
                teleports,
                time,
                plugin_version_id: state.plugin_version_id,
            })
            .await?;

            let reply = Message::reply(&message, message::Outgoing::NewRecordAck {
                record_id: record.record_id,
                pb_data: record.pb_data,
            })
            .encode()?;

            conn.send(reply).await.map_err(Into::into)
        },
    }
}

fn shutdown_close_frame() -> CloseFrame {
    CloseFrame {
        code: close_code::NORMAL,
        reason: "server is shutting down".into(),
    }
}

fn timeout_close_frame() -> CloseFrame {
    CloseFrame {
        code: close_code::POLICY,
        reason: "exceeded heartbeat timeout".into(),
    }
}

fn unauthorized_close_frame() -> CloseFrame {
    CloseFrame {
        code: close_code::POLICY,
        reason: "unauthorized".into(),
    }
}
