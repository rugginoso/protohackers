use std::{
    convert::Infallible,
    task::{Context, Poll},
};

use futures::future::{Ready, ready};
use tower::Service;

use crate::state::{Key, State, Value};

#[derive(Debug, PartialEq)]
pub enum UDPRequest {
    Insert { key: Key, value: Value },
    Retrieve { key: Key },
}

#[derive(Debug, PartialEq)]
pub struct UDPResponse {
    pub key: Key,
    pub value: Value,
}

#[derive(Default)]
pub struct UDPService {
    state: State,
}

impl Service<UDPRequest> for UDPService {
    type Response = Option<UDPResponse>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: UDPRequest) -> Self::Future {
        let resp = match req {
            UDPRequest::Insert { key, value } => {
                self.state.insert(key, value);
                None
            }
            UDPRequest::Retrieve { key } => self.state.retrieve(&key).map(|value| UDPResponse {
                key,
                value: value.clone(),
            }),
        };

        ready(Ok(resp))
    }
}

#[cfg(test)]
mod tests {
    use tower::{Service, ServiceExt};

    use crate::{
        service::{UDPRequest, UDPResponse, UDPService},
        state::{Key, Value},
    };

    #[tokio::test]
    async fn test_insert_retrieve() {
        let mut svc = UDPService::default();
        {
            let svc = ServiceExt::ready(&mut svc).await.unwrap();

            let req = UDPRequest::Insert {
                key: Key::from("foo"),
                value: Value::from("bar"),
            };
            assert!(svc.call(req).await.unwrap().is_none());
        }

        {
            let svc = ServiceExt::ready(&mut svc).await.unwrap();
            let req = UDPRequest::Retrieve {
                key: Key::from("foo"),
            };

            let resp = svc.call(req).await.unwrap();
            assert_eq!(
                resp,
                Some(UDPResponse {
                    key: Key::from("foo"),
                    value: Value::from("bar")
                })
            );
        }
    }
}
