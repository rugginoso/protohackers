use std::{
    convert::Infallible,
    future::{Ready, ready},
    task::{Context, Poll},
};

use tower::Service;

use crate::prime::is_prime;

pub struct PrimeTimeService;

impl Service<PrimeTimeRequest> for PrimeTimeService {
    type Response = PrimeTimeResponse;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: PrimeTimeRequest) -> Self::Future {
        let resp = match req {
            PrimeTimeRequest::IsPrime(Some(n)) => PrimeTimeResponse::IsPrime(is_prime(n)),
            PrimeTimeRequest::IsPrime(None) => PrimeTimeResponse::IsPrime(false),
            PrimeTimeRequest::Invalid => PrimeTimeResponse::Invalid,
        };
        ready(Ok(resp))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrimeTimeRequest {
    IsPrime(Option<u64>),
    Invalid,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrimeTimeResponse {
    IsPrime(bool),
    Invalid,
}

#[cfg(test)]
mod tests {
    use tower::{Service, ServiceExt};

    use crate::service::{PrimeTimeRequest, PrimeTimeResponse, PrimeTimeService};

    #[tokio::test]
    async fn valid_request() {
        let mut svc = PrimeTimeService;
        let svc = ServiceExt::ready(&mut svc)
            .await
            .expect("service expected to be ready");

        let response = svc
            .call(PrimeTimeRequest::IsPrime(Some(7)))
            .await
            .expect("unexpected error");

        assert_eq!(response, PrimeTimeResponse::IsPrime(true));

        let response = svc
            .call(PrimeTimeRequest::IsPrime(Some(10)))
            .await
            .expect("unexpected error");

        assert_eq!(response, PrimeTimeResponse::IsPrime(false));
    }

    #[tokio::test]
    async fn valid_request_with_invalid_number() {
        let mut svc = PrimeTimeService;
        let svc = ServiceExt::ready(&mut svc)
            .await
            .expect("service expected to be ready");

        let response = svc
            .call(PrimeTimeRequest::IsPrime(None))
            .await
            .expect("unexpected error");

        assert_eq!(response, PrimeTimeResponse::IsPrime(false));
    }

    #[tokio::test]
    async fn invalid_request() {
        let mut svc = PrimeTimeService;
        let svc = ServiceExt::ready(&mut svc)
            .await
            .expect("service expected to be ready");

        let response = svc
            .call(PrimeTimeRequest::Invalid)
            .await
            .expect("unexpected error");

        assert_eq!(response, PrimeTimeResponse::Invalid);
    }
}
