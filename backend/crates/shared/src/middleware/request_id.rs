use std::future::{Ready, ready};

use actix_web::{
    Error, HttpMessage,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::header::{HeaderName, HeaderValue},
};
use futures::future::LocalBoxFuture;
use tracing::Instrument;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

tokio::task_local! {
    pub static REQUEST_ID: String;
}

pub fn current() -> Option<String> {
    REQUEST_ID.try_with(|id| id.clone()).ok()
}

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RequestIdMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddlewareService { service }))
    }
}

pub struct RequestIdMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let header_name = HeaderName::from_static(REQUEST_ID_HEADER);

        let incoming = req
            .headers()
            .get(&header_name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let request_id = incoming.unwrap_or_else(|| Uuid::new_v4().to_string());

        req.extensions_mut().insert(RequestId(request_id.clone()));

        let http_req = req.request().clone();
        let method = req.method().clone();
        let path = req.path().to_string();
        let span = tracing::info_span!(
            "request",
            request_id = %request_id,
            method = %method,
            path = %path,
        );
        let scoped_id = request_id.clone();
        let call = self.service.call(req);
        let fut = REQUEST_ID.scope(scoped_id, call.instrument(span));

        Box::pin(async move {
            let header_value = HeaderValue::from_str(&request_id).ok();
            match fut.await {
                Ok(mut res) => {
                    if let Some(value) = header_value {
                        res.headers_mut().insert(header_name, value);
                    }
                    Ok(res.map_into_left_body())
                }
                Err(err) => {
                    let mut http_res = err.error_response();
                    if let Some(value) = header_value {
                        http_res.headers_mut().insert(header_name, value);
                    }
                    let res = ServiceResponse::new(http_req, http_res);
                    Ok(res.map_into_right_body())
                }
            }
        })
    }
}
