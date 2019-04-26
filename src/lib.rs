#![feature(async_await, await_macro)]

use http::{Request, Response};
use hyper::client::HttpConnector;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::await;
use tower::{
    builder::ServiceBuilder, limit::RateLimit, load_shed::LoadShed, timeout::Timeout,
    util::ServiceExt, Service,
};
use tower_test::mock;

pub use tower_hyper::{client::Client, Body};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Serialize, Deserialize, Debug)]
struct Origin {
    origin: String,
}
pub struct Svc<T> {
    inner: Timeout<RateLimit<LoadShed<T>>>,
}

impl<T> Svc<T>
where
    T: Service<Request<Body>>,
{
    pub fn new(svc: T) -> Self {
        let inner = ServiceBuilder::new()
            .timeout(Duration::from_secs(500))
            .rate_limit(100, Duration::from_millis(100))
            .load_shed()
            .service(svc);
        Self { inner }
    }

    pub async fn call(&mut self, req: Request<Body>) -> Result<Response<Body>, Error> {
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

fn new_mock() -> (Mock, Handle) {
    mock::pair()
}

#[test]
fn hello() -> Result<(), Error> {
    let (mut svc, mut handle) = new_mock();
    let ready = svc.poll_ready()?.is_ready();
    assert!(ready);

    let uri: Uri = "http://httpbin.org/ip".parse()?;
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::from(hyper::Body::empty()))?;

    let response = svc.call(req);

    Ok(())
}
