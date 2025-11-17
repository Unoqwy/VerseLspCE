use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, anyhow};
use fxhash::FxHashMap;
use lsp_server::{Message, Notification};
use lsp_types::notification::{Notification as _, PublishDiagnostics};
use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWorkspaceFoldersParams, OneOf, PublishDiagnosticsParams,
    Url, WorkspaceFolder, WorkspaceFoldersServerCapabilities, WorkspaceServerCapabilities,
};

use crate::server::LanguageServer;
use crate::utils;
use crate::verse::{ProjectContainer, SourcePackage};
use crate::vproject::VProjectFile;

pub fn capabilities_workspace_folders() -> WorkspaceServerCapabilities {
    WorkspaceServerCapabilities {
        workspace_folders: Some(WorkspaceFoldersServerCapabilities {
            supported: Some(true),
            change_notifications: Some(OneOf::Left(true)),
        }),
        file_operations: None,
    }
}

impl LanguageServer {
    pub fn handle_notif_workspace_folders_change(
        &mut self,
        params: DidChangeWorkspaceFoldersParams,
    ) -> anyhow::Result<()> {
        let mut removed_project_containers = vec![];
        for workspace_folder in params.event.removed.iter() {
            let extracted = self
                .project_containers
                .extract_if(.., |element| element.workspace_folder.eq(&workspace_folder));
            removed_project_containers.extend(extracted);

            self.workspace_folders
                .retain(|element| element.uri == workspace_folder.uri);
        }

        for project_container in removed_project_containers {
            // TODO: Remove project container
        }

        for workspace_folder in params.event.added.iter() {
            for vproject_path in self.find_vproject_files(&workspace_folder) {
                self.register_project_container(vproject_path, workspace_folder.clone());
            }
        }
        self.workspace_folders.extend(params.event.added);

        self.publish_diagnostics();

        Ok(())
    }

    pub fn handle_notif_document_change(
        &mut self,
        params: DidChangeTextDocumentParams,
    ) -> anyhow::Result<()> {
        let path = self.uri_to_file_path(&params.text_document.uri)?;

        let contents = params
            .content_changes
            .into_iter()
            .nth(0)
            .filter(|c| c.range.is_none())
            .context("Expected full document due to FULL sync mode")?
            .text;

        for project_container in self.project_containers.iter_mut() {
            for package in project_container.packages.clone() {
                if path.starts_with(&package.dir_path) {
                    project_container.update_source(&package, &path, &contents);
                }
            }

            project_container.build();
        }

        self.publish_diagnostics();

        Ok(())
    }

    fn find_vproject_files(&self, workspace_folder: &WorkspaceFolder) -> Vec<PathBuf> {
        if let Ok(path) = workspace_folder.uri.to_file_path() {
            utils::collect_files_with_extension(&path, "vproject")
        } else {
            vec![]
        }
    }

    fn register_project_container(
        &mut self,
        vproject_path: PathBuf,
        workspace_folder: WorkspaceFolder,
    ) {
        let Ok(vproject_uri) = Url::from_file_path(&vproject_path) else {
            log::error!("Unable to turn .vproject file path to URI: {vproject_path:?}");
            return;
        };

        let read_vproject_file = || -> anyhow::Result<VProjectFile> {
            let contents = fs::read_to_string(&vproject_path)?;
            let vproject_file: VProjectFile = serde_json::from_str(&contents)?;
            Ok(vproject_file)
        };
        let vproject_file = match read_vproject_file() {
            Ok(parsed) => parsed,
            Err(err) => {
                log::error!(
                    "Unable to read/parse .vprojet file to register project container: {err}"
                );
                return;
            }
        };

        let c_container = crate::register_project_container(&workspace_folder.name);

        let mut packages = vec![];
        for package in vproject_file.packages.iter() {
            let Ok(dir_path) = PathBuf::from(&package.desc.dir_path).canonicalize() else {
                continue;
            };
            let c_package = crate::register_package(
                &c_container,
                package.desc.name.as_str(),
                package.desc.dir_path.as_str(),
                package.read_only,
                &package.desc.settings,
            );
            packages.push(Rc::new(SourcePackage {
                name: package.desc.name.clone(),
                verse_path: package.desc.settings.verse_path.clone(),
                dir_path,
                c_package,
            }));
        }

        let project_container = ProjectContainer {
            workspace_folder,
            vproject_uri,
            vproject_file,
            c_container,
            packages,
            diagnostics: Default::default(),
            stale_diagnostic_uris: Default::default(),
            file_cache: Default::default(),
        };
        self.project_containers.push(project_container);

        let project_container = self.project_containers.last_mut().expect("Just pushed");
        project_container.load_files_from_disk();
        project_container.build();
    }

    fn publish_diagnostics(&mut self) {
        let mut all_diagnostics = FxHashMap::default();
        for project_container in self.project_containers.iter_mut() {
            if !project_container.stale_diagnostic_uris.is_empty() {
                for stale_uri in std::mem::take(&mut project_container.stale_diagnostic_uris) {
                    all_diagnostics
                        .entry(stale_uri.clone())
                        .or_insert_with(|| vec![]);
                }
            }
            for (uri, diagnostics) in project_container.diagnostics.iter() {
                all_diagnostics
                    .entry(uri.clone())
                    .or_insert_with(|| vec![])
                    .extend(diagnostics.clone());
            }
        }
        for (uri, diagnostics) in all_diagnostics {
            self.connection
                .sender
                .send(Message::Notification(Notification::new(
                    PublishDiagnostics::METHOD.to_owned(),
                    PublishDiagnosticsParams {
                        uri: uri.clone(),
                        diagnostics,
                        version: None,
                    },
                )))
                .unwrap();
        }
    }
}
