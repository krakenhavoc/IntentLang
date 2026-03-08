use tower_lsp::{LspService, Server};

mod completion;
mod diagnostics;
mod document;
mod hover;
mod navigation;
mod server;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(server::Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
