use std::{
    convert::Infallible,
    future::{Ready, ready},
    ops::RangeInclusive,
    task::Poll,
};

use tower::Service;

use crate::state::{self, Price, State, Timestamp};

#[derive(Default)]
pub struct MeansToAnEndService {
    state: State,
}

impl Service<MeansToAnEndRequest> for MeansToAnEndService {
    type Response = MeansToAnEndResponse;
    type Error = Infallible;

    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: MeansToAnEndRequest) -> Self::Future {
        let resp = match req {
            MeansToAnEndRequest::Insert { timestamp, price } => {
                self.state.insert(timestamp, price);
                MeansToAnEndResponse::Empty
            }
            MeansToAnEndRequest::Query(time_range) => {
                let mean = self.state.aggregate::<state::Mean>(time_range);
                MeansToAnEndResponse::Mean(mean)
            }
        };
        ready(Ok(resp))
    }
}

#[derive(Debug, PartialEq)]
pub enum MeansToAnEndRequest {
    Insert { timestamp: Timestamp, price: Price },
    Query(RangeInclusive<Timestamp>),
}

#[derive(Debug, PartialEq)]
pub enum MeansToAnEndResponse {
    Mean(Price),
    Empty,
}

#[cfg(test)]
mod tests {
    use tower::{Service, ServiceExt};

    use crate::{
        service::{MeansToAnEndRequest, MeansToAnEndResponse, MeansToAnEndService},
        state::{Price, Timestamp},
    };

    #[tokio::test]
    async fn test_insert() {
        let mut svc = MeansToAnEndService::default();
        let svc = ServiceExt::ready(&mut svc)
            .await
            .expect("expected service to be ready");

        let resp = svc
            .call(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(0),
                price: Price::new(10),
            })
            .await
            .expect("unexpected error");

        assert_eq!(resp, MeansToAnEndResponse::Empty);
    }

    #[tokio::test]
    async fn test_query_empty() {
        let mut svc = MeansToAnEndService::default();
        let svc = ServiceExt::ready(&mut svc)
            .await
            .expect("expected service to be ready");

        let resp = svc
            .call(MeansToAnEndRequest::Query(
                Timestamp::new(0)..=Timestamp::new(10),
            ))
            .await
            .expect("unexpected error");

        assert_eq!(resp, MeansToAnEndResponse::Mean(Price::new(0)));
    }

    #[tokio::test]
    async fn test_query_some_values() {
        let mut svc = MeansToAnEndService::default();
        let svc = ServiceExt::ready(&mut svc)
            .await
            .expect("expected service to be ready");

        let _ = svc
            .call(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(0),
                price: Price::new(10),
            })
            .await
            .expect("unexpected error");

        let _ = svc
            .call(MeansToAnEndRequest::Insert {
                timestamp: Timestamp::new(10),
                price: Price::new(20),
            })
            .await
            .expect("unexpected error");

        let resp = svc
            .call(MeansToAnEndRequest::Query(
                Timestamp::new(0)..=Timestamp::new(30),
            ))
            .await
            .expect("unexpected error");

        assert_eq!(resp, MeansToAnEndResponse::Mean(Price::new(15)));
    }
}
