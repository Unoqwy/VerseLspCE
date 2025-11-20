use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

use anyhow::Context;
use lsp_server::{self, Message, RequestId, Response};
use lsp_server::{ErrorCode, ResponseError};

use lsp_types::notification::*;
use lsp_types::request::*;
use lsp_types::*;

use crate::server::LanguageServer;

macro_rules! message_type_def {
(
    $tyname:ident,

    ($tylspsrc:path, $method_trait:path),

    $(
        $tyvariant:ident($tyvariantparams:ty) => $handle_method:ident
    ),*
    $(,)?
) => {
    #[derive(Debug)]
    pub enum $tyname {
    $(
        $tyvariant($tyvariantparams),
    )*
    }

    impl $tyname {
        fn parse(src: $tylspsrc) -> anyhow::Result<Option<Self>> {
            use $method_trait;

            let parsed = match src.method.as_str() {
            $(
                $tyvariant::METHOD => Self::$tyvariant(serde_json::from_value(src.params)?),
            )*
                _ => return Ok(None),
            };
            Ok(Some(parsed))
        }

        impl_handlers!($tyname, $($tyvariant => $handle_method,)*);
    }
};
}

macro_rules! impl_handlers {
(
    ParsedRequest,
    $($tyvariant:ident => $handle_method:ident),* ,
) => {
    fn route(self, server: &mut LanguageServer) -> Result<Option<serde_json::Value>, ResponseError> {
        match self {
        $(
            Self::$tyvariant(params) => server.$handle_method(params)
                .map_err(|err| ResponseError {
                    code: ErrorCode::RequestFailed as i32,
                    message: format!("{:?}", err),
                    data: None,
                })
                .and_then(|result| {
                    serde_json::to_value(&result).map_err(|err| ResponseError {
                        code: ErrorCode::RequestFailed as i32,
                        message: format!("{}", err),
                        data: None,
                    })
                })
                .map(Some),
        )*
        }
    }
};
(
    ParsedNotification,
    $($tyvariant:ident => $handle_method:ident),* ,
) => {
    fn route(self, server: &mut LanguageServer) -> anyhow::Result<()> {
        match self {
        $(
            Self::$tyvariant(params) => server.$handle_method(params),
        )*
        }
    }
};
}

#[derive(Debug)]
pub enum ParsedMessage {
    Request(ParsedRequest),
    Notification(ParsedNotification),
}

message_type_def!(
    ParsedRequest,
    (lsp_server::Request, lsp_types::request::Request),
    SemanticTokensFullRequest(SemanticTokensParams) => handle_req_semantic_tokens_full,
);

message_type_def!(
    ParsedNotification,
    (lsp_server::Notification, lsp_types::notification::Notification),
    DidChangeWorkspaceFolders(DidChangeWorkspaceFoldersParams) => handle_did_workspace_folders_change,
    DidChangeTextDocument(DidChangeTextDocumentParams) => handle_did_document_change,
);

#[derive(Debug)]
pub struct MessageQueue {
    pub queue: Mutex<VecDeque<QueuedMessage>>,
    pub condvar: Condvar,
}

/// We'll try to be clever about queued messages and dedup requests to the same file,
/// though this will cause certain requests to be cancelled.
#[derive(Debug)]
pub struct QueuedMessage {
    /// Request ID when [`Self::message`] is a [`ParsedMessage::Request`].
    pub req_id: Option<RequestId>,
    /// Received message from the client.
    pub message: ParsedMessage,

    /// Resolved relevant URIs from [`Self::message`].
    pub uris: Vec<Url>,

    /// Requires the project to be compiled to be processed.
    pub compile_gated: bool,
}

/// Messages are processed in a separate thread than the one receiving them.
/// This worker thread gets blocked by project building, but tries to be clever
/// about it by compiling only after processing all queued messages that warrant a new build.
/// TODO: Extra debouncing based on .build() average time
pub fn message_processing_worker(mut server: LanguageServer) -> anyhow::Result<()> {
    let message_queue = server.message_queue.clone();
    loop {
        let mut queue = message_queue.queue.lock().unwrap();

        let msg = queue.pop_front();

        if msg.as_ref().map(|msg| msg.compile_gated).unwrap_or(true) {
            let mut any_built = false;
            for project_container in server.project_containers.iter_mut() {
                if project_container.needs_build {
                    project_container.build();
                    any_built = true;
                }
            }
            if any_built {
                server.publish_diagnostics();
            }
        }

        let Some(msg) = msg else {
            while queue.is_empty() {
                queue = message_queue.condvar.wait(queue).unwrap();
            }
            continue;
        };

        std::mem::drop(queue);

        log::debug!("Processing: {msg:?}");
        match msg.message {
            ParsedMessage::Request(req) => {
                let req_id = msg.req_id.context("Request must have an ID")?;
                let response = match req.route(&mut server) {
                    Ok(result) => Response {
                        id: req_id,
                        result: result,
                        error: None,
                    },
                    Err(err) => Response {
                        id: req_id,
                        result: None,
                        error: Some(err),
                    },
                };
                server.connection.sender.send(Message::Response(response))?;
            }
            ParsedMessage::Notification(notification) => {
                if let Err(err) = notification.route(&mut server) {
                    log::error!("Notification error: {err:?}");
                }
            }
        }
    }
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(Default::default()),
            condvar: Condvar::new(),
        }
    }

    pub fn cancel_request(&self, cancel_req_id: NumberOrString) {
        let cancel_req_id = match cancel_req_id {
            NumberOrString::Number(id) => RequestId::from(id),
            NumberOrString::String(string_id) => RequestId::from(string_id),
        };

        let mut queue = self.queue.lock().unwrap();
        queue.retain(|message| {
            if let Some(req_id) = &message.req_id
                && req_id.eq(&cancel_req_id)
            {
                false
            } else {
                true
            }
        });
    }

    pub fn queue_message(&self, message: Message) -> anyhow::Result<()> {
        let req_id = match &message {
            Message::Request(req) => Some(req.id.clone()),
            _ => None,
        };

        let Some(message) = (match message {
            Message::Request(req) => ParsedRequest::parse(req)?.map(ParsedMessage::Request),
            Message::Notification(notification) => {
                ParsedNotification::parse(notification)?.map(ParsedMessage::Notification)
            }
            _ => None,
        }) else {
            return Ok(());
        };

        let mut uris = Vec::with_capacity(1);
        let mut compile_gated = false;
        match &message {
            ParsedMessage::Request(req) => match req {
                ParsedRequest::SemanticTokensFullRequest(params) => {
                    uris.push(params.text_document.uri.clone());
                    compile_gated = true;
                }
            },
            ParsedMessage::Notification(notification) => match notification {
                ParsedNotification::DidChangeTextDocument(params) => {
                    uris.push(params.text_document.uri.clone());
                }
                _ => {}
            },
        }

        // TODO: Dedup

        let mut queue = self.queue.lock().unwrap();
        queue.push_back(QueuedMessage {
            req_id,
            message,
            uris,
            compile_gated,
        });

        self.condvar.notify_one();

        Ok(())
    }
}
