#![feature(async_await, await_macro)]

use http::{Method, Request, Response, StatusCode, Uri};
use hyper::client::HttpConnector;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::{
    await,
    prelude::{Async, Poll},
};
use tower::{
    builder::ServiceBuilder, limit::RateLimit, load_shed::LoadShed, timeout::Timeout,
    util::ServiceExt, Service,
};
use tower_test::{assert_request_eq, mock};

pub use tower_hyper::{client::Client, Body};

type Error = Box<dyn std::error::Error + Send + Sync>;

#[derive(Serialize, Deserialize, Debug)]
struct Origin {
    origin: String,
}

pub struct Svc<T>
where
    T: Service<Request<Body>>,
    T::Error: Into<Error>,
{
    inner: Timeout<RateLimit<LoadShed<T>>>,
}

impl<T, R> Svc<T>
where
    T: Service<R>,
    T::Error: Into<Error>,
{
    pub fn new(svc: T) -> Self {
        let inner = ServiceBuilder::new()
            .timeout(Duration::from_secs(500))
            .rate_limit(100, Duration::from_millis(100))
            .load_shed()
            .service(svc);
        Self { inner }
    }

    pub async fn call(&mut self, req: R) -> Result<T::Response, Error> {
        let svc = &mut self.inner;
        await!(svc.ready())?;
        await!(svc.call(req))
    }
}

impl Default for Svc<Client<HttpConnector, Body>> {
    fn default() -> Self {
        Svc::new(Client::new())
    }
}

type Mock = mock::Mock<Request<Body>, Response<Body>>;
type Handle = mock::Handle<Request<Body>, Response<Body>>;

fn new_mock() -> (Svc<Mock>, Handle) {
    let (svc, handle) = mock::pair();
    (Svc::new(svc), handle)
}

#[test]
fn hello() -> Result<(), Error> {
    let (mut svc, mut handle) = new_mock();

    let uri: Uri = "http://httpbin.org/ip".parse()?;
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::from(hyper::Body::empty()))?;

    let mut response = svc.call(req);

    let send_response = Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(hyper::Body::empty()))?;

    Ok(())
}
