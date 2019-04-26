#![feature(async_await, await_macro)]

use http::{Method, Request, Uri};
use serde::{Deserialize, Serialize};
use tokio::await;
use tokio_buf::util::BufStreamExt;
use tower_client::{Body, Svc};
use tower_http::BodyExt;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Serialize, Deserialize, Debug)]
struct Origin {
    origin: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let uri: Uri = "http://httpbin.org/ip".parse()?;

    let mut svc = Svc::default();
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::from(hyper::Body::empty()))?;

    let (headers, body) = await!(svc.call(req))?.into_parts();
    let body = await!(parse_body(body))?;

    dbg!(headers);
    dbg!(body);

    Ok(())
}

async fn parse_body(body: Body) -> Result<Origin, Error> {
    let body = await!(body.into_buf_stream().collect::<Vec<u8>>()).unwrap();
    let body = String::from_utf8(body)?;
    let origin = serde_json::from_str(&body)?;
    Ok(origin)
}
