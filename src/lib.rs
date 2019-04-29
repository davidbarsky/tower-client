#![feature(async_await, await_macro)]

use http::Request;
use hyper::client::HttpConnector;
use std::time::Duration;
use tokio::await;
use tower::{
    builder::ServiceBuilder, limit::RateLimit, load_shed::LoadShed, timeout::Timeout,
    util::ServiceExt, Service,
};

pub use tower_hyper::{client::Client, Body};

type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct Svc<T, R>
where
    T: Service<R>,
    T::Error: Into<Error>,
{
    inner: Timeout<RateLimit<LoadShed<T>>>,
    _phan: std::marker::PhantomData<R>,
}

impl<T, R> Svc<T, R>
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
        Self {
            inner,
            _phan: std::marker::PhantomData,
        }
    }

    pub async fn call(&mut self, req: R) -> Result<T::Response, Error> {
        let svc = &mut self.inner;
        await!(svc.ready())?;
        await!(svc.call(req))
    }
}

impl Default for Svc<Client<HttpConnector, Body>, Request<Body>> {
    fn default() -> Self {
        Svc::new(Client::new())
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        await,
        prelude::future::{poll_fn},
    };
    use tower_test::mock;

    use crate::{Error, Svc};

    type Mock = mock::Mock<String, String>;
    type Handle = mock::Handle<String, String>;

    fn new_mock() -> (Svc<Mock, String>, Handle) {
        let (svc, handle) = mock::pair();
        (Svc::new(svc), handle)
    }

    #[test]
    fn hello() -> Result<(), Error> {
        let (mut svc, mut handle) = new_mock();
        let req = String::from("hello, ");

        let mut rt = tokio::runtime::Runtime::new().unwrap();

        let fut = async move {
            dbg!("Sending req");
            let resp = svc.call(req);

            // on the side of the "server"...
            dbg!("polling readiness of handle");
            let pair = poll_fn(move || handle.poll_request());
            dbg!("awaiting poll");
            let pair = await!(pair).unwrap();
            dbg!("accepted req");
            let (req, handle) = pair.unwrap();
            assert_eq!(req, String::from("hello, "));
            handle.send_response(String::from("world!"));

            let resp = await!(resp).unwrap();
            assert_eq!(resp, String::from("world!"));
        };

        rt.block_on_async(fut);

        Ok(())
    }
}
