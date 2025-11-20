use std::ffi::{c_char, c_void};

use crate::{features::semantic_tokens::SemanticTokensAccumulator, verse::DiagnosticAccumulator};

#[repr(C)]
#[derive(Debug)]
pub struct SSourceSpan {
    pub begin_row: u32,
    pub begin_col: u32,
    pub end_row: u32,
    pub end_col: u32,
}

#[repr(C)]
pub struct LspProjectContainer(c_void);

#[repr(C)]
pub struct SPackage(c_void);

#[repr(C)]
pub struct SPackageSettings {
    pub verse_path: *const c_char,
    pub verse_scope: u8,
    pub role: u8,
    pub explicit_verse_version: bool,
    pub verse_version: u32,
    pub fortnite_version: u32, // 0 = Latest
    pub treat_modules_as_implicit: bool,
    pub dependency_packages: *const *const c_char,
    pub dependency_packages_len: usize,
    pub vni_dest_dir: *const c_char,
    pub allow_experimental: bool,
}

#[repr(C)]
pub struct SDiagnostic {
    pub path: *const c_char,
    pub message: *const c_char,
    pub reference_code: u16,
    pub severity: i32,
    pub span: SSourceSpan,
}

unsafe extern "C" {
    #![allow(improper_ctypes)]

    pub fn Lsp_RegisterProjectContainer(project_name: *const c_char) -> *mut LspProjectContainer;

    pub fn Lsp_Build(
        project_container: *mut LspProjectContainer,
        diagnostics: *mut DiagnosticAccumulator,
    );

    pub fn Lsp_RegisterPackage(
        project_container: *const LspProjectContainer,
        package_name: *const c_char,
        dir_path: *const c_char,
        read_only: bool,
        settings: SPackageSettings,
    ) -> *const SPackage;

    pub fn Lsp_UnregisterPackage(package: *const SPackage);

    pub fn Lsp_UpsertSource(
        package: *const SPackage,
        path: *const c_char,
        module_path_to_root: *const c_char,
        contents: *const c_char,
    );

    pub fn Lsp_SemanticTokens(
        project_container: *mut LspProjectContainer,
        package: *const SPackage,
        path: *const c_char,
        semantic_tokens: *const SemanticTokensAccumulator,
    );
}
