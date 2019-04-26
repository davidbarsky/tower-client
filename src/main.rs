#![feature(async_await, await_macro)]

use http::{Method, Request, Response, Uri};
use hyper::client::HttpConnector;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::await;
use tokio_buf::util::BufStreamExt;
use tower::{
    builder::ServiceBuilder, limit::RateLimit, load_shed::LoadShed, timeout::Timeout,
    util::ServiceExt, Service,
};
use tower_http::BodyExt;
use tower_hyper::{client::Client, Body};
use tower_test::{assert_request_eq, mock};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let uri: Uri = "http://httpbin.org/ip".parse()?;

    let mut svc = Svc::new(Client::new());
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::from(hyper::Body::empty()))?;

    let res = await!(svc.call(req))?;
    println!("{:?}", res);

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct Origin {
    origin: String,
}

type InnerSvc = Timeout<RateLimit<LoadShed<Client<HttpConnector, Body>>>>;
struct Svc {
    inner: InnerSvc,
}

impl Svc {
    fn new(svc: Client<HttpConnector, Body>) -> Self {
        let inner = ServiceBuilder::new()
            .timeout(Duration::from_secs(500))
            .rate_limit(100, Duration::from_millis(100))
            .load_shed()
            .service(svc);
        Self { inner }
    }

    async fn call(&mut self, req: Request<Body>) -> Result<Response<Origin>, Error> {
        let svc = &mut self.inner;

        await!(svc.ready())?;
        let (headers, body) = await!(svc.call(req))?.into_parts();
        let body = await!(body.into_buf_stream().collect::<Vec<u8>>()).unwrap();
        let body = String::from_utf8(body)?;
        let origin: Origin = serde_json::from_str(&body)?;

        Ok(Response::from_parts(headers, origin))
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

    let mut response = svc.call(req);

    Ok(())
}
