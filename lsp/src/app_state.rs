use std::sync::Arc;

use anyhow::Result;
use common::config::Config;
use common::sync::GithubClient;
use common::sync::WebDavClient;
#[cfg(not(test))]
use tower_lsp::Client as LspClient;

#[cfg(test)]
use crate::mocks::MockLspClient as LspClient;
#[cfg(test)]
use crate::watching::MockPathStore as PathStore;
#[cfg(not(test))]
use crate::watching::PathStore;

#[derive(Debug)]
pub struct AppState {
    pub watched_paths: PathStore,
}

impl AppState {
    pub fn new(config: &Config, lsp_client: Arc<LspClient>) -> Result<Self> {
        let sync_client: Arc<dyn common::sync::Client> = if config.is_webdav() {
            Arc::new(WebDavClient::new(
                config.webdav_url().to_string(),
                config.webdav_username().to_string(),
                config.webdav_password().to_string(),
                config.webdav_remote_path().to_string(),
            )?)
        } else {
            Arc::new(GithubClient::new(
                config.gist_id().to_string(),
                config.github_token().to_string(),
            )?)
        };
        let watched_paths = PathStore::new(sync_client, lsp_client)?;

        Ok(Self { watched_paths })
    }
}
