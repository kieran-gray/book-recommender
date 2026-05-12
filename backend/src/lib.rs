pub mod api;
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod setup;

use serde_json::json;
use tracing::error;

use worker::*;

use crate::{
    api::create_router,
    setup::{AppState, Config},
};

#[event(start)]
fn start() {}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let config = match Config::from_env(&env) {
        Ok(config) => config,
        Err(err) => {
            error!(error = ?err, "Failed to create config");
            let json = json!({"message": format!("Failed to create config: {err}")});
            return Ok(Response::from_json(&json)?.with_status(500));
        }
    };

    let app_state = match AppState::from_env(&env, config) {
        Ok(app_state) => app_state,
        Err(err) => {
            error!(error = ?err, "Failed to create app state");
            let json = json!({"message": format!("Failed to create app state: {err}")});
            return Ok(Response::from_json(&json)?.with_status(500));
        }
    };

    let router = create_router(app_state);
    let result = router.run(req, env).await;

    if let Err(e) = &result {
        error!(error = ?e, "Worker request failed");
    }

    result
}
