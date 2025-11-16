use lsp_server::{ErrorCode, ResponseError};
use lsp_types::*;

use crate::server::LanguageServer;

pub fn capabilities_semantic_tokens() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: Some(false),
        },
        legend: SemanticTokensLegend {
            token_types: vec![],
            token_modifiers: vec![],
        },
        range: Some(false),
        full: Some(SemanticTokensFullOptions::Bool(true)),
    })
}

impl LanguageServer {
    pub fn handle_req_semantic_tokens_full(
        &mut self,
        params: SemanticTokensParams,
    ) -> Result<SemanticTokensFullDeltaResult, ResponseError> {
        let path = self
            .uri_to_file_path(&params.text_document.uri)
            .map_err(|err| ResponseError {
                code: ErrorCode::RequestFailed as i32,
                message: format!("{err:?}"),
                data: None,
            })?;
        let path_str = path.to_string_lossy();

        for project_container in self.project_containers.iter_mut() {
            for package in project_container.packages.iter() {
                if path.starts_with(&package.dir_path) {
                    crate::symbol_info(
                        &project_container.c_container,
                        &package.c_package,
                        &path_str,
                    );
                }
            }
        }

        Ok(SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        }))
    }
}
