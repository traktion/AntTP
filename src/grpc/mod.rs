pub mod pointer_handler;

use std::future::{ready, Ready};
use actix_web::dev::{Service as ActixService, ServiceRequest, ServiceResponse, ServiceFactory, HttpServiceFactory};
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use tonic::server::NamedService;
use tonic::body::BoxBody;
use actix_web::body::BoxBody as ActixBoxBody;
use tonic::codegen::Service as TonicServiceTrait;

pub type Never = std::convert::Infallible;

#[derive(Clone)]
pub struct TonicService<S> {
    service: S,
}

impl<S> TonicService<S> {
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S> HttpServiceFactory for TonicService<S>
where
    S: TonicServiceTrait<tonic::codegen::http::Request<BoxBody>, Response = tonic::codegen::http::Response<BoxBody>, Error = Never> + NamedService + Clone + 'static,
    S::Future: 'static,
{
    fn register(self, config: &mut actix_web::dev::AppService) {
        actix_web::web::scope(S::NAME)
            .service(TonicServiceWrapper { service: self.service })
            .register(config);
    }
}

pub struct TonicServiceWrapper<S> {
    service: S,
}

impl<S> HttpServiceFactory for TonicServiceWrapper<S>
where
    S: TonicServiceTrait<tonic::codegen::http::Request<BoxBody>, Response = tonic::codegen::http::Response<BoxBody>, Error = Never> + Clone + 'static,
    S::Future: 'static,
{
    fn register(self, config: &mut actix_web::dev::AppService) {
        config.register_service(
            actix_web::dev::ResourceDef::new(""),
            None,
            TonicServiceFactory { service: self.service },
            None,
        );
    }
}

pub struct TonicServiceFactory<S> {
    service: S,
}

impl<S> ServiceFactory<ServiceRequest> for TonicServiceFactory<S>
where
    S: TonicServiceTrait<tonic::codegen::http::Request<BoxBody>, Response = tonic::codegen::http::Response<BoxBody>, Error = Never> + Clone + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<ActixBoxBody>;
    type Error = Error;
    type Config = ();
    type Service = TonicServiceMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Service, Self::InitError>>;

    fn new_service(&self, _: Self::Config) -> Self::Future {
        ready(Ok(TonicServiceMiddleware {
            service: self.service.clone(),
        }))
    }
}

pub struct TonicServiceMiddleware<S> {
    service: S,
}

impl<S> ActixService<ServiceRequest> for TonicServiceMiddleware<S>
where
    S: TonicServiceTrait<tonic::codegen::http::Request<BoxBody>, Response = tonic::codegen::http::Response<BoxBody>, Error = Never> + Clone + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<ActixBoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        TonicServiceTrait::poll_ready(&mut self.service.clone(), cx).map_err(|_| unreachable!())
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let (_request, _payload) = req.into_parts();
        Box::pin(async move {
            Err(actix_web::error::ErrorInternalServerError("gRPC integration requires careful payload conversion"))
        })
    }
}
