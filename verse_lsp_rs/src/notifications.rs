use lsp_types::notification::*;

use crate::server::LanguageServer;

macro_rules! notif_handlers {
    ($notif:expr, $($notif_ty:ty => $self:ident.$fn:ident),* $(,)?) => {
        match $notif.method.as_str() {
            $(
                <$notif_ty>::METHOD => $self.$fn(serde_json::from_value($notif.params)?),
            )*
            _ => return Ok(())
        }
    };
}

impl LanguageServer {
    pub fn handle_notification(&mut self, notif: lsp_server::Notification) -> anyhow::Result<()> {
        notif_handlers! {
            notif,
            DidChangeWorkspaceFolders => self.handle_notif_workspace_folders_change,
            DidChangeTextDocument => self.handle_notif_document_change,
        }
    }
}
