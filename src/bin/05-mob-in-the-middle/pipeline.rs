use std::borrow::Cow;

use futures::{SinkExt, StreamExt, future::ready};

use crate::substitute::substitute_boguscoin_address;

#[derive(Debug, PartialEq)]
pub enum Event {
    Client(String),
    Server(String),
    ClientClose,
    ServerClose,
}

pub fn client_stream<S, E>(rx: S) -> impl futures::Stream<Item = Event>
where
    S: futures::Stream<Item = Result<String, E>>,
{
    rx.map(|res| match res {
        Ok(msg) => Event::Client(msg),
        Err(_) => Event::ClientClose,
    })
    .take_while(|e| ready(!matches!(e, Event::ClientClose)))
    .chain(futures::stream::once(async { Event::ClientClose }))
}

pub fn server_stream<S, E>(rx: S) -> impl futures::Stream<Item = Event>
where
    S: futures::Stream<Item = Result<String, E>>,
{
    rx.map(|res| match res {
        Ok(msg) => Event::Server(msg),
        Err(_) => Event::ServerClose,
    })
    .take_while(|e| ready(!matches!(e, Event::ServerClose)))
    .chain(futures::stream::once(async { Event::ServerClose }))
}

pub async fn run<Events, ClientSink, ServerSink, E>(
    mut events: Events,
    mut client_sink: ClientSink,
    mut server_sink: ServerSink,
    replacement: &str,
) -> Result<(), E>
where
    Events: futures::Stream<Item = Event> + Unpin,
    ClientSink: futures::Sink<String, Error = E> + Unpin,
    ServerSink: futures::Sink<String, Error = E> + Unpin,
{
    while let Some(event) = events.next().await {
        match event {
            Event::Client(msg) => {
                let msg = substitute_boguscoin_address(&msg, replacement);
                server_sink.send(msg.into_owned()).await?;
            }
            Event::Server(msg) => {
                let msg = if msg.starts_with('*') {
                    Cow::Borrowed(msg.as_str())
                } else {
                    substitute_boguscoin_address(&msg, replacement)
                };
                client_sink.send(msg.into_owned()).await?;
            }
            Event::ClientClose | Event::ServerClose => {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use crate::pipeline::{Event, client_stream};

    #[tokio::test]
    async fn test_client_stream_ok() {
        let input: Vec<Result<String, String>> =
            vec![Ok(String::from("first")), Ok(String::from("second"))];

        let stream = client_stream(futures::stream::iter(input));
        assert_eq!(
            stream.collect::<Vec<Event>>().await,
            vec![
                Event::Client("first".into()),
                Event::Client("second".into()),
                Event::ClientClose
            ]
        );
    }

    #[tokio::test]
    async fn test_client_stream_err() {
        let input: Vec<Result<String, String>> = vec![
            Ok(String::from("first")),
            Err(String::from("error")),
            Ok(String::from("second")),
        ];

        let stream = client_stream(futures::stream::iter(input));
        assert_eq!(
            stream.collect::<Vec<Event>>().await,
            vec![Event::Client("first".into()), Event::ClientClose,]
        );
    }

    mod pipeline {
        use crate::pipeline::{self, Event};

        #[tokio::test]
        async fn test_client_events_are_forwarded_to_server() {
            let mut client_sink: Vec<String> = Vec::new();
            let mut server_sink: Vec<String> = Vec::new();

            let events = vec![
                Event::Client(String::from(
                    "Please send the payment of 750 Boguscoins to 7PM5y5HTsk8cwYxc2C6jflP2SkvNMO",
                )),
                Event::Client(String::from(
                    "Payement to 7PM5y5HTsk8cwYxc2C6jflP2SkvNMO sent",
                )),
                Event::ClientClose,
            ];

            assert!(
                pipeline::run(
                    futures::stream::iter(events),
                    &mut client_sink,
                    &mut server_sink,
                    "7replacementaddress123456789012345"
                )
                .await
                .is_ok()
            );
            assert!(client_sink.is_empty());
            assert_eq!(
                server_sink,
                vec![
                    String::from(
                        "Please send the payment of 750 Boguscoins to 7replacementaddress123456789012345"
                    ),
                    String::from("Payement to 7replacementaddress123456789012345 sent"),
                ]
            );
        }

        #[tokio::test]
        async fn test_server_events_are_forwarded_to_client() {
            let mut client_sink: Vec<String> = Vec::new();
            let mut server_sink: Vec<String> = Vec::new();

            let events = vec![
                Event::Server(String::from(
                    "* 7PM5y5HTsk8cwYxc2C6jflP2SkvNMO joined the chat",
                )),
                Event::Server(String::from(
                    "[7PM5y5HTsk8cwYxc2C6jflP2SkvNMO] Payement to 7PM5y5HTsk8cwYxc2C6jflP2SkvNMO sent",
                )),
                Event::ServerClose,
            ];

            assert!(
                pipeline::run(
                    futures::stream::iter(events),
                    &mut client_sink,
                    &mut server_sink,
                    "7replacementaddress123456789012345"
                )
                .await
                .is_ok()
            );
            assert!(server_sink.is_empty());
            assert_eq!(
                client_sink,
                vec![
                    String::from("* 7PM5y5HTsk8cwYxc2C6jflP2SkvNMO joined the chat"),
                    String::from(
                        "[7PM5y5HTsk8cwYxc2C6jflP2SkvNMO] Payement to 7replacementaddress123456789012345 sent"
                    ),
                ]
            );
        }
    }
}
