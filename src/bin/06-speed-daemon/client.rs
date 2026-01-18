use std::{
    collections::HashSet,
    fmt::Display,
    sync::{Arc, Mutex},
};

use futures::{SinkExt, StreamExt, future::pending};
use tokio::{sync::mpsc, time::Instant};
use tokio_util::codec::Framed;

use crate::{
    codec::{IncomingMessage, OutgoingMessage, SpeedDaemonCodec},
    state::State,
};

pub async fn handle_client(stream: tokio::net::TcpStream, state: Arc<Mutex<State>>) {
    Client::new(stream).run(state).await;
}

enum HeartBeat {
    Unset,
    ZeroInterval,
    Set {
        join_handle: tokio::task::JoinHandle<()>,
        rx: mpsc::Receiver<()>,
    },
}

impl HeartBeat {
    fn set(&mut self, interval: std::time::Duration) -> Result<(), &'static str> {
        if matches!(self, HeartBeat::ZeroInterval | HeartBeat::Set { .. }) {
            return Err("heartbeat already started");
        }

        if interval.is_zero() {
            *self = HeartBeat::ZeroInterval;
            return Ok(());
        }

        let (tx, rx) = mpsc::channel(1);

        let join_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval_at(Instant::now() + interval, interval);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let _ = tx.send(()).await;
                    }
                    _ = tx.closed() => {
                        break;
                    }
                }
            }
        });

        *self = HeartBeat::Set { join_handle, rx };
        Ok(())
    }

    async fn tick(&mut self) -> Option<()> {
        match self {
            HeartBeat::Unset | HeartBeat::ZeroInterval => pending().await,
            HeartBeat::Set { rx, .. } => rx.recv().await,
        }
    }
}

impl Drop for HeartBeat {
    fn drop(&mut self) {
        match self {
            HeartBeat::Unset | HeartBeat::ZeroInterval => {}
            HeartBeat::Set { join_handle, rx } => {
                rx.close();
                join_handle.abort();
            }
        }
    }
}

struct Unauthenticated;

struct Client<S> {
    inner: S,
    framed: Framed<tokio::net::TcpStream, SpeedDaemonCodec>,
    heartbeat: HeartBeat,
}

impl<S> Client<S> {
    fn start_heartbeat(&mut self, interval: std::time::Duration) -> Result<(), &'static str> {
        self.heartbeat.set(interval)
    }

    fn client_id(&self) -> impl Display {
        self.framed
            .get_ref()
            .peer_addr()
            .expect("get peer addr failed")
    }
}

impl Client<Unauthenticated> {
    fn new(stream: tokio::net::TcpStream) -> Self {
        Self {
            inner: Unauthenticated,
            framed: Framed::new(stream, SpeedDaemonCodec),
            heartbeat: HeartBeat::Unset,
        }
    }

    pub async fn run(mut self, state: Arc<Mutex<State>>) {
        loop {
            tokio::select! {
                msg = self.framed.next() => {
                    match msg {
                        None => {
                            break;
                        },
                        Some(Err(err)) => {
                            tracing::error!("decode error from client {}: {}", self.client_id(), err);
                            let _ = self
                                .framed
                                .send(OutgoingMessage::Error(format!(
                                    "decode error: {err}"
                                )))
                                .await;
                            break;
                        },
                        Some(Ok(IncomingMessage::WantHeartBeat{ interval })) => {
                            if let Err(err) = self.start_heartbeat(interval) {
                                tracing::error!("failed to start heartbeat for client {}: {}", self.client_id(), err);
                                let _ = self
                                    .framed
                                    .send(OutgoingMessage::Error(format!(
                                        "failed to start heartbeat: {err}"
                                    )))
                                    .await;
                            }
                        },
                        Some(Ok(IncomingMessage::IAmCamera{ road_id, mile, limit })) => {
                            return self.into_camera(road_id, mile, limit).run(state).await;
                        },
                        Some(Ok(IncomingMessage::IAmDispatcher{ roads_ids })) => {
                            return self.into_dispatcher(roads_ids).run(state).await;
                        },
                        _ => {
                            self.framed
                                .send(OutgoingMessage::Error("unexpected message".to_string()))
                                .await
                                .expect("send error message failed");
                            break;
                        }
                    }
                },
                _ = self.heartbeat.tick() => {
                    if let Err(err) = self.framed.send(OutgoingMessage::HeartBeat).await {
                        tracing::error!("failed to send heartbeat to client {}: {}", self.client_id(), err);
                        break;
                    }
                },
            }
        }
    }

    fn into_camera(
        self,
        road_id: crate::state::RoadID,
        mile: crate::state::MileMarker,
        limit: crate::state::MphX100,
    ) -> Client<Camera> {
        Client {
            inner: Camera {
                road_id,
                mile,
                limit,
            },
            framed: self.framed,
            heartbeat: self.heartbeat,
        }
    }

    fn into_dispatcher(self, roads_ids: Vec<crate::state::RoadID>) -> Client<TicketDispatcher> {
        Client {
            inner: TicketDispatcher {
                roads_ids: roads_ids.into_iter().collect(),
            },
            framed: self.framed,
            heartbeat: self.heartbeat,
        }
    }
}

struct Camera {
    road_id: crate::state::RoadID,
    mile: crate::state::MileMarker,
    limit: crate::state::MphX100,
}

impl Client<Camera> {
    async fn run(mut self, state: Arc<Mutex<State>>) {
        state.lock().expect("lock failed").add_camera(
            self.client_id().into(),
            self.inner.road_id,
            self.inner.mile,
            self.inner.limit,
        );

        loop {
            tokio::select! {
                msg = self.framed.next() => {
                    match msg {
                        None => {
                            break;
                        },
                        Some(Err(err)) => {
                            tracing::error!("decode error from client {}: {}", self.client_id(), err);
                            let _ = self
                                .framed
                                .send(OutgoingMessage::Error(format!(
                                    "decode error: {err}"
                                )))
                                .await;
                            break;
                        },
                        Some(Ok(IncomingMessage::WantHeartBeat{ interval })) => {
                            if let Err(err) = self.start_heartbeat(interval) {
                                tracing::error!("failed to start heartbeat for client {}: {}", self.client_id(), err);
                                let _ = self
                                    .framed
                                    .send(OutgoingMessage::Error(format!(
                                        "failed to start heartbeat: {err}"
                                    )))
                                    .await;
                            }
                        },
                        Some(Ok(IncomingMessage::Plate{ plate, timestamp })) => {
                            state.lock().expect("lock failed").add_observation(
                                self.client_id().into(),
                                plate,
                                timestamp,
                            );
                        },
                        _ => {
                            self.framed
                                .send(OutgoingMessage::Error("unexpected message".to_string()))
                                .await
                                .expect("send error message failed");
                            break;
                        }
                    }
                },
                _ = self.heartbeat.tick() => {
                    if let Err(err) = self.framed.send(OutgoingMessage::HeartBeat).await {
                        tracing::error!("failed to send heartbeat to client {}: {}", self.client_id(), err);
                        break;
                    }
                },
            }
        }

        state
            .lock()
            .expect("lock failed")
            .remove_camera(self.client_id().into());
    }
}

struct TicketDispatcher {
    roads_ids: HashSet<crate::state::RoadID>,
}

impl Client<TicketDispatcher> {
    async fn run(mut self, state: Arc<Mutex<State>>) {
        let mut check_tickets_interval = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {
                msg = self.framed.next() => {
                    match msg {
                        None => {
                            break;
                        },
                        Some(Err(err)) => {
                            tracing::error!("decode error from client {}: {}", self.client_id(), err);
                            let _ = self
                                .framed
                                .send(OutgoingMessage::Error(format!(
                                    "decode error: {err}"
                                )))
                                .await;
                            break;
                        },
                        Some(Ok(IncomingMessage::WantHeartBeat{ interval })) => {
                            if let Err(err) = self.start_heartbeat(interval) {
                                tracing::error!("failed to start heartbeat for client {}: {}", self.client_id(), err);
                                let _ = self
                                    .framed
                                    .send(OutgoingMessage::Error(format!(
                                        "failed to start heartbeat: {err}"
                                    )))
                                    .await;
                            }
                        },
                        _ => {
                            self.framed
                                .send(OutgoingMessage::Error("unexpected message".to_string()))
                                .await
                                .expect("send error message failed");
                            break;
                        }
                    }
                },
                _ = self.heartbeat.tick() => {
                    if let Err(err) = self.framed.send(OutgoingMessage::HeartBeat).await {
                        tracing::error!("failed to send heartbeat to client {}: {}", self.client_id(), err);
                    }
                },
                _ = check_tickets_interval.tick() => {
                    let tickets = state.lock().expect("lock failed").get_tickets_for_roads(&self.inner.roads_ids);
                    for ticket in tickets {
                        if let Err(err) = self.framed.send(OutgoingMessage::Ticket(ticket)).await {
                            tracing::error!("failed to send ticket to client {}: {}", self.client_id(), err);
                            break;
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::HeartBeat;

    #[tokio::test]
    async fn test_heartbeat_first_tick_waits_interval() {
        let mut heartbeat = HeartBeat::Unset;
        heartbeat.set(Duration::from_millis(100)).unwrap();

        assert!(timeout(Duration::from_millis(40), heartbeat.tick()).await.is_err());
        assert_eq!(
            timeout(Duration::from_millis(140), heartbeat.tick())
                .await
                .unwrap(),
            Some(())
        );
    }

    #[tokio::test]
    async fn test_heartbeat_zero_interval_never_ticks() {
        let mut heartbeat = HeartBeat::Unset;
        heartbeat.set(Duration::ZERO).unwrap();

        assert!(timeout(Duration::from_millis(40), heartbeat.tick()).await.is_err());
    }

    #[tokio::test]
    async fn test_heartbeat_cannot_be_started_twice() {
        let mut heartbeat = HeartBeat::Unset;
        heartbeat.set(Duration::from_millis(100)).unwrap();

        assert!(heartbeat.set(Duration::from_millis(100)).is_err());
    }
}
