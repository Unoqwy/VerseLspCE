use std::sync::Arc;

use lsp_server::Connection;
use lsp_types::WorkspaceFolder;

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
}
