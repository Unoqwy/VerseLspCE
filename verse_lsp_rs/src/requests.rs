use lsp_server::{ErrorCode, Response, ResponseError};

use lsp_types::request::Request as _;
use lsp_types::request::*;

use crate::server::LanguageServer;

macro_rules! req_handlers {
    ($req:expr, $($req_ty:ty => $self:ident.$fn:ident),* $(,)?) => {
        match $req.method.as_str() {
            $(
            <$req_ty>::METHOD => $self.$fn(serde_json::from_value($req.params)?)
                .and_then(|result| {
                    serde_json::to_value(&result).map_err(|err| ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: format!("{}", err),
                        data: None,
                    })
                }),
            )*
            _ => return Ok(None)
        }
    };
}

impl LanguageServer {
    pub fn handle_request(&mut self, req: lsp_server::Request) -> anyhow::Result<Option<Response>> {
        let result = req_handlers! {
            req,
            SemanticTokensFullRequest => self.handle_req_semantic_tokens_full,
        };
        let resp = match result {
            Ok(result) => Response::new_ok(req.id.clone(), result),
            Err(err) => Response {
                id: req.id.clone(),
                result: None,
                error: Some(err),
            },
        };
        Ok(Some(resp))
    }
}

fn req_failed<S: ToString>(message: S) -> ResponseError {
    ResponseError {
        code: ErrorCode::RequestFailed as i32,
        message: message.to_string(),
        data: None,
    }
}
