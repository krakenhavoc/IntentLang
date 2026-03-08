//! LSP backend: routes requests to diagnostics, hover, completion, and navigation.

use std::path::PathBuf;

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completion;
use crate::diagnostics;
use crate::document::Document;
use crate::hover;
use crate::navigation;

/// The LSP backend. Holds per-file state in a concurrent map.
pub struct Backend {
    client: Client,
    documents: DashMap<Url, Document>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Backend {
            client,
            documents: DashMap::new(),
        }
    }

    /// Parse, check, and publish diagnostics for a document.
    async fn update_document(&self, uri: Url, text: String) {
        let file_path = uri_to_path(&uri);
        let doc = Document::new(text, file_path.as_deref());
        let diags = diagnostics::compute_diagnostics(&doc, &uri);
        self.documents.insert(uri.clone(), doc);
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..CompletionOptions::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "IntentLang LSP server initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.update_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // With FULL sync, the last content change contains the full text.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.update_document(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Re-check on save in case imports changed on disk.
        if let Some(text) = params.text {
            self.update_document(params.text_document.uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let offset = doc.line_index.position_to_offset(pos, &doc.source);
        Ok(hover::hover_at(&doc, offset))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let offset = doc.line_index.position_to_offset(pos, &doc.source);
        Ok(completion::completions(&doc, offset))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let offset = doc.line_index.position_to_offset(pos, &doc.source);
        Ok(navigation::goto_definition(&doc, offset, uri))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Convert a `file://` URI to a filesystem path.
fn uri_to_path(uri: &Url) -> Option<PathBuf> {
    if uri.scheme() == "file" {
        uri.to_file_path().ok()
    } else {
        None
    }
}
