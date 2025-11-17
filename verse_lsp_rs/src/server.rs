use std::{path::PathBuf, sync::Arc};

use lsp_server::Connection;
use lsp_types::{Url, WorkspaceFolder};

use anyhow::anyhow;

use crate::verse::ProjectContainer;

pub struct LanguageServer {
    /// LSP connection.
    pub connection: Arc<Connection>,
    /// Workspace folders of the LSP client, unrelated to actual Verse project folders.
    pub workspace_folders: Vec<WorkspaceFolder>,
    /// Each .vproject file gets its own project container, aka. server workspace.
    pub project_containers: Vec<ProjectContainer>,
}

impl LanguageServer {
    pub fn new(connection: Arc<Connection>) -> Self {
        Self {
            connection,
            workspace_folders: vec![],
            project_containers: vec![],
        }
    }

    pub fn uri_to_file_path(&self, uri: &Url) -> anyhow::Result<PathBuf> {
        uri.to_file_path()
            .map_err(|_| anyhow!("Text document URI couldn't be mapped to file path: {uri}"))?
            .canonicalize()
            .map_err(|err| err.into())
    }

    pub fn normalize_uri(&self, uri: &Url) -> anyhow::Result<Url> {
        Url::from_file_path(self.uri_to_file_path(uri)?)
            .map_err(|_| anyhow!("Text document path couldn't be mapped to URI: {uri}"))
    }
}
