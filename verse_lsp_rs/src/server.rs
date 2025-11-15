use std::{
    collections::HashSet,
    path::Path,
    sync::{Arc, OnceLock},
};

use lsp_server::Connection;
use lsp_types::WorkspaceFolder;
use walkdir::WalkDir;

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

pub(crate) fn walk_files<F>(path: &Path, mut f: F)
where
    F: FnMut(walkdir::DirEntry) -> (),
{
    /// Common directory names to ignore when walking file tree.
    /// Mainly speeds things up by ignoring the usually massive `__ExternalActors__`
    /// that doesn't contain any .verse files.
    static IGNORE_DIRECTORIES: OnceLock<HashSet<&'static str>> = OnceLock::new();

    let ignore_directories = IGNORE_DIRECTORIES.get_or_init(|| {
        HashSet::from([".git", ".urc", "__ExternalActors__", "__ExternalObjects__"])
    });

    let mut it = WalkDir::new(path).into_iter();
    loop {
        let dir_entry = match it.next() {
            Some(Ok(dir_entry)) => dir_entry,
            Some(Err(_)) => continue,
            None => break,
        };

        if let Some(file_name) = dir_entry.file_name().to_str()
            && ignore_directories.contains(file_name)
        {
            it.skip_current_dir();
            continue;
        }

        f(dir_entry)
    }
}
