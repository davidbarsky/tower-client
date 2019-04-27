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
