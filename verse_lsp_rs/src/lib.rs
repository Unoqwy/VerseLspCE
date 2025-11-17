#![allow(special_module_name, dead_code)]

use std::ffi::{CStr, CString, c_char};

use crate::{
    features::semantic_tokens::{SemanticTokenEntry, SemanticTokensAccumulator},
    verse::{CProjectContainer, CSourcePackage, DiagnosticAccumulator},
};
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Url};

use simple_logger::SimpleLogger;

mod entrypoint;
mod features;
mod ffi;
mod notifications;
mod requests;
mod server;
pub mod utils;
mod verse;
mod vproject;

#[unsafe(no_mangle)]
pub extern "C" fn RS_RunServer() -> i32 {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .unwrap();

    return match entrypoint::main() {
        Ok(_) => 0,
        Err(err) => {
            log::error!("Server stopped with error: {err}");
            1
        }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn RS_AddDiagnostic(acc: *mut DiagnosticAccumulator, diagnostic: ffi::SDiagnostic) {
    let acc = unsafe { &mut *acc };

    let path = unsafe { CStr::from_ptr(diagnostic.path) }
        .to_string_lossy()
        .into_owned();
    let path = if path.is_empty() {
        None
    } else {
        match Url::from_file_path(&path) {
            Ok(path) => Some(path),
            Err(_) => {
                log::error!("Couldn't convert path \"{path}\" to url");
                return;
            }
        }
    };

    let message = unsafe { CStr::from_ptr(diagnostic.message) }
        .to_string_lossy()
        .into_owned();

    let span = diagnostic.span;
    let diagnostic = Diagnostic {
        range: Range::new(
            Position::new(span.begin_row, span.begin_col),
            Position::new(span.end_row, span.end_col),
        ),
        severity: Some(match diagnostic.severity {
            1 => DiagnosticSeverity::ERROR,
            2 => DiagnosticSeverity::WARNING,
            3 => DiagnosticSeverity::INFORMATION,
            _ => unimplemented!("Diagnostic severity code"),
        }),
        code: if diagnostic.reference_code > 0 {
            Some(NumberOrString::Number(diagnostic.reference_code as _))
        } else {
            None
        },
        source: Some("VerseCompiler".to_owned()),
        message,
        ..Default::default()
    };

    if let Some(path) = path {
        acc.diagnostics
            .entry(path)
            .or_insert_with(|| vec![])
            .push(diagnostic);
    } else {
        acc.global_diagnostics.push(diagnostic);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn RS_AddSemanticToken(
    acc: *mut SemanticTokensAccumulator,
    token_entry: SemanticTokenEntry,
) {
    let acc = unsafe { &mut *acc };

    acc.token_entries.push(token_entry);
}

pub fn register_project_container(project_name: &str) -> CProjectContainer {
    let c_project_name = CString::new(project_name).unwrap();
    let ptr = unsafe { ffi::Lsp_RegisterProjectContainer(c_project_name.as_ptr()) };
    CProjectContainer(ptr)
}

pub fn build(project_container: &CProjectContainer, diagnostics: &mut DiagnosticAccumulator) {
    unsafe {
        ffi::Lsp_Build(project_container.0, diagnostics);
    }
}

pub fn register_package(
    project_container: &CProjectContainer,
    package_name: &str,
    dir_path: &str,
    read_only: bool,
    settings: &vproject::PackageSettings,
) -> CSourcePackage {
    let c_package_name = CString::new(package_name).unwrap();
    let c_dir_path = CString::new(dir_path).unwrap();
    let s_verse_path = CString::new(settings.verse_path.as_str()).unwrap();

    let c_vni_dest_dir = settings
        .vni_dest_dir
        .as_ref()
        .and_then(|dir| CString::new(dir.as_str()).ok());

    let c_dependency_packages_owned: Vec<CString> = settings
        .dependency_packages
        .iter()
        .flat_map(|s| CString::new(s.as_str()).ok())
        .collect();
    let c_dependency_packages: Vec<*const c_char> = c_dependency_packages_owned
        .iter()
        .map(|s| s.as_ptr())
        .collect();
    let c_settings = ffi::SPackageSettings {
        verse_path: s_verse_path.as_ptr(),
        verse_scope: settings.verse_scope as u8,
        role: settings.role as u8,
        explicit_verse_version: settings.verse_version.is_some(),
        verse_version: settings.verse_version.unwrap_or(0),
        treat_modules_as_implicit: settings.treat_modules_as_implicit,
        dependency_packages: c_dependency_packages.as_ptr(),
        dependency_packages_len: settings.dependency_packages.len(),
        vni_dest_dir: if let Some(vni_dest_dir) = c_vni_dest_dir {
            vni_dest_dir.as_ptr()
        } else {
            std::ptr::null()
        },
        allow_experimental: settings.allow_experimental,
    };
    let ptr = unsafe {
        ffi::Lsp_RegisterPackage(
            project_container.0,
            c_package_name.as_ptr(),
            c_dir_path.as_ptr(),
            read_only,
            c_settings,
        )
    };
    CSourcePackage(ptr)
}

pub fn unregister_package(package: CSourcePackage) {
    unsafe {
        ffi::Lsp_UnregisterPackage(package.0);
    };
}

pub fn upsert_source(
    package: &CSourcePackage,
    path: &str,
    module_path_to_root: &str,
    contents: &str,
) {
    let c_path = CString::new(path).unwrap();
    let c_module_path_to_root = CString::new(module_path_to_root).unwrap();
    let c_contents = CString::new(contents).unwrap();
    unsafe {
        ffi::Lsp_UpsertSource(
            package.0,
            c_path.as_ptr(),
            c_module_path_to_root.as_ptr(),
            c_contents.as_ptr(),
        );
    };
}

pub fn get_semantic_tokens(
    project_container: &CProjectContainer,
    package: &CSourcePackage,
    path: &str,
    semantic_tokens: &mut SemanticTokensAccumulator,
) {
    let c_path = CString::new(path).unwrap();
    unsafe {
        ffi::Lsp_SemanticTokens(
            project_container.0,
            package.0,
            c_path.as_ptr(),
            semantic_tokens,
        );
    };
}
