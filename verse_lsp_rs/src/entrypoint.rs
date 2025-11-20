use std::{sync::Arc, thread};

use anyhow::{Context, bail};
use clap::Parser;
use lsp_server::{self, Connection, IoThreads, Message};

use lsp_types::{
    CancelParams, DidChangeWorkspaceFoldersParams, InitializeParams, InitializeResult, OneOf,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkspaceFoldersChangeEvent,
};

use crate::server::VerseLspCESettings;
use crate::{
    features::{
        semantic_tokens::capabilities_semantic_tokens, workspace::capabilities_workspace_folders,
    },
    server::{self, LanguageServer, messages::MessageQueue},
};

#[derive(clap::Parser)]
struct Cli {
    #[command(flatten)]
    transport: Transport,

    /// Whether to keep running the server after a TCP client disconnects and rebind the listener.
    #[arg(long, default_value_t = false)]
    forever: bool,
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

pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(address) = cli.transport.tcp {
        loop {
            let (connection, io_threads) = Connection::listen(&address)?;
            handle_client(connection, io_threads)?;

            if !cli.forever {
                break;
            }
        }

        Ok(())
    } else {
        let (connection, io_threads) = Connection::stdio();
        handle_client(connection, io_threads)
    }
}

fn server_config() -> InitializeResult {
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

    InitializeResult {
        capabilities: server_capabilities,
        server_info: Some(ServerInfo {
            name: "VerseLspCE".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
        ..Default::default()
    }
}

fn handle_client(connection: Connection, io_threads: IoThreads) -> anyhow::Result<()> {
    let server_init_payload = serde_json::to_value(server_config())
        .context("Couldn't serialize server initialize result")?;

    let (init_id, init_params) = connection.initialize_start()?;
    connection.initialize_finish(init_id, server_init_payload)?;

    let client_init_params: InitializeParams =
        serde_json::from_value(init_params).context("Couldn't parse initialize params")?;
    let mut settings = match client_init_params.initialization_options {
        Some(json) => {
            serde_json::from_value(json).context("Couldn't parse custom VerseLspCE user options")?
        }
        None => VerseLspCESettings::default(),
    };

    // if Fortnite version is not explicitely specified,
    // we want the actual Latest version rather than whatever the server was compiled with
    // TODO: Find latest local UEFN version
    if settings.fortnite_version.is_none() {
        settings.fortnite_version = Some(3811);
    }

    let connection = Arc::new(connection);
    let message_queue = Arc::new(MessageQueue::new());

    thread::spawn({
        let connection = connection.clone();
        let message_queue = message_queue.clone();

        move || {
            let mut server = LanguageServer::new(connection, message_queue, settings);

            // add default workspace folders
            if let Some(workspace_folders) = client_init_params.workspace_folders {
                let _ =
                    server.handle_did_workspace_folders_change(DidChangeWorkspaceFoldersParams {
                        event: WorkspaceFoldersChangeEvent {
                            added: workspace_folders,
                            ..Default::default()
                        },
                    });
            }

            if let Err(err) = server::messages::message_processing_worker(server) {
                log::error!("Message processing failed: {err}");
            }
        }
    });

    for msg in &connection.receiver {
        if let Message::Request(req) = &msg
            && connection.handle_shutdown(req)?
        {
            break;
        }

        if let Message::Notification(notification) = &msg
            && notification.method.eq("$/cancelRequest")
        {
            let Message::Notification(notification) = msg else {
                bail!("Notification should be Notification");
            };
            let params: CancelParams = serde_json::from_value(notification.params)?;
            message_queue.cancel_request(params.id);
            continue;
        }

        log::debug!("Queueing: {msg:?}");
        if let Err(err) = message_queue.queue_message(msg) {
            log::error!("Failed to queue message: {err}");
        }
    }

    io_threads.join()?;

    Ok(())
}
