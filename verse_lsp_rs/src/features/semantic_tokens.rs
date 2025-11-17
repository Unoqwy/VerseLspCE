use enum_iterator::Sequence;
use lsp_server::{ErrorCode, ResponseError};
use lsp_types::*;

use crate::{
    ffi,
    server::LanguageServer,
    verse::{ProjectContainer, SourcePackage},
};

#[repr(u32)]
#[derive(Clone, Copy, Debug, Sequence, PartialEq, Eq)]
pub enum SemanticTokenKind {
    Namespace,
    Type,
    Enum,
    EnumMember,
    Struct,
    Class,
    Interface,
    Parameter,
    TypeParameter,
    Property,
    Variable,
    Function,
    Method,
    Macro,
    Keyword,
    Comment,
    String,
    Number,
    Operator,
    Attribute,
    Specifier,
}

impl SemanticTokenKind {
    pub fn to_lsp_type_id(self) -> u32 {
        self as u32
    }

    pub fn to_lsp_type_def(self) -> SemanticTokenType {
        match self {
            Self::Namespace => SemanticTokenType::NAMESPACE,
            Self::Type => SemanticTokenType::TYPE,
            Self::Enum => SemanticTokenType::ENUM,
            Self::EnumMember => SemanticTokenType::ENUM_MEMBER,
            Self::Struct => SemanticTokenType::STRUCT,
            Self::Class => SemanticTokenType::CLASS,
            Self::Interface => SemanticTokenType::INTERFACE,
            Self::Parameter => SemanticTokenType::PARAMETER,
            Self::TypeParameter => SemanticTokenType::TYPE_PARAMETER,
            Self::Property => SemanticTokenType::PROPERTY,
            Self::Variable => SemanticTokenType::VARIABLE,
            Self::Function => SemanticTokenType::FUNCTION,
            Self::Method => SemanticTokenType::METHOD,
            Self::Macro => SemanticTokenType::MACRO,
            Self::Keyword => SemanticTokenType::KEYWORD,
            Self::Comment => SemanticTokenType::COMMENT,
            Self::String => SemanticTokenType::STRING,
            Self::Number => SemanticTokenType::NUMBER,
            Self::Operator => SemanticTokenType::OPERATOR,
            Self::Attribute => SemanticTokenType::new("attribute"),
            Self::Specifier => SemanticTokenType::new("specifier"),
        }
    }
}

pub fn capabilities_semantic_tokens() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions {
            work_done_progress: Some(false),
        },
        legend: SemanticTokensLegend {
            token_types: enum_iterator::all::<SemanticTokenKind>()
                .map(SemanticTokenKind::to_lsp_type_def)
                .collect(),
            token_modifiers: vec![],
        },
        range: Some(false),
        full: Some(SemanticTokensFullOptions::Bool(true)),
    })
}

#[repr(C)]
#[derive(Debug)]
pub struct SemanticTokenEntry {
    pub token_kind: SemanticTokenKind,
    pub span: ffi::SSourceSpan,
}

#[derive(Debug)]
pub struct SemanticTokensAccumulator {
    pub token_entries: Vec<SemanticTokenEntry>,
}

impl LanguageServer {
    pub fn handle_req_semantic_tokens_full(
        &self,
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

        let mut semantic_tokens = vec![];
        for project_container in self.project_containers.iter() {
            for package in project_container.packages.iter() {
                if path.starts_with(&package.dir_path) {
                    semantic_tokens = self.get_semantic_tokens(
                        project_container,
                        package,
                        &params.text_document.uri,
                        &path_str,
                    );
                    break;

                    // TODO: Decide how to handle files shared by multiple packages or projects, if ever relevant
                }
            }
        }

        Ok(SemanticTokensFullDeltaResult::Tokens(SemanticTokens {
            result_id: None,
            data: semantic_tokens,
        }))
    }

    fn get_semantic_tokens(
        &self,
        project_container: &ProjectContainer,
        package: &SourcePackage,
        uri: &Url,
        path_str: &str,
    ) -> Vec<SemanticToken> {
        let Some(file_state) = project_container.file_cache.get(uri) else {
            // TODO: Turn into request error
            log::error!("Missing file cache for {path_str}");
            return vec![];
        };

        let mut acc = SemanticTokensAccumulator {
            token_entries: vec![],
        };

        crate::get_semantic_tokens(
            &project_container.c_container,
            &package.c_package,
            &path_str,
            &mut acc,
        );

        acc.token_entries
            .sort_unstable_by_key(|entry| (entry.span.begin_row, entry.span.begin_col));

        let mut output_tokens = Vec::with_capacity(acc.token_entries.len());

        let span_source = &file_state.span_source;
        let mut last_line = 0;
        let mut last_col = 0;
        for entry in acc.token_entries {
            let Some((start, end)) = span_source.span_to_byte_offsets(&entry.span) else {
                continue;
            };
            let length = end - start;

            let line = entry.span.begin_row;
            let col = entry.span.begin_col;

            let delta_line = line - last_line;
            let delta_start = if delta_line == 0 { col - last_col } else { col };
            output_tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: entry.token_kind.to_lsp_type_id(),
                token_modifiers_bitset: 0,
            });
            last_line = line;
            last_col = col;
        }

        output_tokens
    }
}
