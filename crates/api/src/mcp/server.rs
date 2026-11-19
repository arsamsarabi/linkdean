//! MCP server handler. Tools land in later tasks; empty list is intentional for the scaffold.

use rmcp::handler::server::ServerHandler;
use rmcp::model::{Implementation, ProtocolVersion, ServerCapabilities, ServerConfig};

#[derive(Clone, Default)]
pub struct LinkdeanMcp;

impl ServerHandler for LinkdeanMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("linkdean", env!("CARGO_PKG_VERSION")))
            .with_protocol_version(ProtocolVersion::V_2025_03_26)
            .with_instructions(
                "Linkdean read-only library tools. Search and read bookmarks; no writes."
                    .to_string(),
            )
    }
}
