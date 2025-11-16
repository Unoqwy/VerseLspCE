use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use fxhash::FxHashMap;
use lsp_types::{Diagnostic, Url, WorkspaceFolder};

use crate::{ffi, profile, utils, vproject::VProjectFile};

#[derive(Debug)]
pub struct CProjectContainer(pub *mut ffi::LspProjectContainer);

/// Contains all LSP data about a Verse project bound to a .vproject file.
#[derive(Debug)]
pub struct ProjectContainer {
    /// The workspace `vproject_path` originates from.
    pub workspace_folder: WorkspaceFolder,
    /// .vproject file uri.
    pub vproject_uri: Url,
    /// Parsed .vproject file.
    pub vproject_file: VProjectFile,

    /// Pointer to a cpp `LspProjectContainer`.
    pub c_container: CProjectContainer,
    /// Packages.
    pub packages: Vec<SourcePackage>,

    /// Diagnostics from the last build attempt.
    pub diagnostics: FxHashMap<Url, Vec<Diagnostic>>,
    /// Files that need to be cleared of diagnostics.
    pub stale_diagnostic_uris: HashSet<Url>,
}

#[derive(Debug)]
pub struct CSourcePackage(pub *const ffi::SPackage);

#[derive(Debug)]
pub struct SourcePackage {
    pub name: String,
    pub verse_path: String,
    pub dir_path: PathBuf,
    pub c_package: CSourcePackage,
}

pub struct DiagnosticAccumulator {
    /// Diagnostics to report as coming from .vproject file.
    pub global_diagnostics: Vec<Diagnostic>,
    /// Diagnostics bound to URIs.
    pub diagnostics: FxHashMap<Url, Vec<Diagnostic>>,
}

impl ProjectContainer {
    pub fn build(&mut self) {
        let mut diagnostic_acc = DiagnosticAccumulator {
            global_diagnostics: vec![],
            diagnostics: FxHashMap::default(),
        };

        profile! {
            format!("Build project {}", &self.vproject_uri.as_str()),
            crate::build(&self.c_container, &mut diagnostic_acc);
        };

        let mut stale_diagnostic_uris = HashSet::with_capacity(self.diagnostics.len());
        stale_diagnostic_uris.extend(self.diagnostics.keys().cloned());

        self.diagnostics = diagnostic_acc.diagnostics;

        if !diagnostic_acc.global_diagnostics.is_empty() {
            self.diagnostics
                .entry(self.vproject_uri.clone())
                .or_insert_with(|| vec![])
                .extend(diagnostic_acc.global_diagnostics);
        }

        stale_diagnostic_uris.retain(|uri| !self.diagnostics.contains_key(&uri));
        self.stale_diagnostic_uris.extend(stale_diagnostic_uris);
    }

    pub fn load_files_from_disk(&mut self) {
        for package in self.packages.iter() {
            profile! {
                format!("Read package {} files from disk", &package.name),
                self.load_package_files_from_disk(package);
            };
        }
    }

    fn load_package_files_from_disk(&self, package: &SourcePackage) {
        let verse_file_paths = utils::collect_files_with_extension(&package.dir_path, "verse");

        for path in verse_file_paths {
            let Ok(path) = path.canonicalize() else {
                continue;
            };

            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(err) => {
                    log::error!("Unable to read snippet file \"{path:?}\": {err}");
                    continue;
                }
            };
            self.update_source(package, &path, &contents);
        }
    }

    pub fn update_source(&self, package: &SourcePackage, path: &Path, contents: &str) {
        let mut module_path_to_root = "";
        if let Some(parent) = path.parent()
            && let Some(ee) = parent
                .strip_prefix(&package.dir_path)
                .ok()
                .and_then(|p| p.to_str())
        {
            module_path_to_root = ee;
        }

        let path_str = path.to_string_lossy();
        crate::upsert_source(
            &package.c_package,
            &path_str,
            &module_path_to_root,
            &contents,
        );
    }
}
