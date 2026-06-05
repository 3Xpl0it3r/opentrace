// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::future::Future;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use serde::de::DeserializeOwned;

use crate::errors::AgntError;
use crate::manager::Manager;

pub trait ApiResource: Send + Sync + 'static {
    type Request: DeserializeOwned + Send + 'static;

    fn path_prefix() -> &'static str;

    fn start(
        manager: Arc<Manager>,
        req: Self::Request,
    ) -> impl Future<Output = Result<(), AgntError>> + Send;

    fn stop(
        manager: Arc<Manager>,
        name: String,
    ) -> impl Future<Output = Result<(), AgntError>> + Send;
}

pub trait ApiRouter {
    fn with_resource<T: ApiResource>(self, manager: Arc<Manager>) -> Self;
}

impl ApiRouter for Router {
    fn with_resource<T: ApiResource>(self, manager: Arc<Manager>) -> Self {
        let prefix = T::path_prefix();
        let start_route = format!("/start/{}", prefix);
        let stop_route = format!("/stop/{}/{{name}}", prefix);

        let resource_router = Router::<Arc<Manager>>::new()
            .route(
                &start_route,
                post(
                    move |State(manager): State<Arc<Manager>>,
                          Json(req): Json<serde_json::Value>| async move {
                        let req: T::Request = serde_json::from_value(req)
                            .map_err(|e| AgntError::BadRequest(format!("invalid request: {e}")))?;
                        T::start(manager, req).await?;
                        Ok::<_, AgntError>(StatusCode::CREATED)
                    },
                ),
            )
            .route(
                &stop_route,
                post(
                    move |State(manager): State<Arc<Manager>>,
                          Path(name): Path<String>| async move {
                        T::stop(manager, name).await?;
                        Ok::<_, AgntError>(StatusCode::OK)
                    },
                ),
            );

        self.merge(resource_router.with_state(manager))
    }
}
