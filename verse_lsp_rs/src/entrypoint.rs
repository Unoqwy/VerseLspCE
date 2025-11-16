use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use clap::Parser;
use lsp_server::{self, Connection, Message};

use lsp_types::{
    DidChangeWorkspaceFoldersParams, InitializeParams, InitializeResult, OneOf, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, WorkspaceFoldersChangeEvent,
};

use crate::{
    features::{
        semantic_tokens::capabilities_semantic_tokens, workspace::capabilities_workspace_folders,
    },
    server::LanguageServer,
};

#[derive(clap::Parser)]
struct Cli {
    #[command(flatten)]
    transport: Transport,
}

#[derive(clap::Args)]
#[group(multiple = false)]
struct Transport {
    /// Use stdio transport (default)
    #[arg(long)]
    stdio: bool,

    /// Use TCP transport (e.g --tcp 127.0.0.1:9010)
    #[arg(long, value_name = "address")]
    tcp: Option<String>,
}

pub fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    let cli = Cli::parse();

    let (connection, io_threads) = if let Some(address) = cli.transport.tcp {
        // useful to debug the server with gdb
        Connection::listen(address)?
    } else {
        Connection::stdio()
    };

    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        definition_provider: Some(OneOf::Left(true)),
        // document_symbol_provider: Some(OneOf::Left(true)),
        semantic_tokens_provider: Some(capabilities_semantic_tokens()),
        // hover_provider: Some(HoverProviderCapability::Simple(true)),
        workspace: Some(capabilities_workspace_folders()),
        // workspace_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    };

    let initialize_result = InitializeResult {
        capabilities: server_capabilities,
        server_info: Some(ServerInfo {
            name: "VerseLspCE".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
        ..Default::default()
    };

    let initialize_data = serde_json::to_value(initialize_result)
        .expect("Couldn't serialize server initialize result");
    let (init_id, init_params) = connection.initialize_start()?;
    connection.initialize_finish(init_id, initialize_data)?;

    let init_params: InitializeParams =
        serde_json::from_value(init_params).expect("Couldn't parse initialize paramas");

    let connection = Arc::new(connection);
    let mut server = LanguageServer::new(connection.clone());

    if let Some(workspace_folders) = init_params.workspace_folders {
        let _ = server.handle_notif_workspace_folders_change(DidChangeWorkspaceFoldersParams {
            event: WorkspaceFoldersChangeEvent {
                added: workspace_folders,
                ..Default::default()
            },
        });
    }

    let server = Arc::new(Mutex::new(server));
    for msg in &connection.receiver {
        log::debug!("Received: {msg:?}");
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let mut server = server.lock().unwrap();
                match server.handle_request(req) {
                    Ok(Some(resp)) => {
                        connection.sender.send(Message::Response(resp))?;
                    }
                    Ok(None) => {} // unhandled request type
                    Err(err) => log::error!("Unable to parse request: {err:?}"),
                };
            }
            Message::Response(_) => {}
            Message::Notification(notif) => {
                let mut server = server.lock().unwrap();
                if let Err(err) = server.handle_notification(notif.clone()) {
                    log::error!("Notification error: {err:?}");
                }
            }
        }
    }

    io_threads.join()?;

    Ok(())
}
