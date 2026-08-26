// --- Plugin Metadata -----------------------------------------------------------
//
// Every plugin should expose a `MetadataProvider` implementation so the host
// and frontend can surface security/privacy/decentralization information to
// users.  Adjust the values below to reflect your plugin's real posture.

/// Marker type for the plugin template's metadata.
impl tlock_hdk::MetadataProvider for TemplatePluginMetadata {
    fn metadata(&self) -> tlock_hdk::PluginMetadata {
        tlock_hdk::PluginMetadata::new()
            .open_source(true)
            .audited_contract(false)
            .safe_harbor(false)
            .bug_bounty(false)
            .centralized_infrastructure(false)
            .reports_data(false)
            .source_url("https://github.com/Robert-MacWha/lodgelock")
            .license("MIT")
            .version("0.1.0")
            .description("Plugin template for the LodgeLock ecosystem.")
    }
}

/// Convenience accessor — returns the template plugin's metadata.
    TemplatePluginMetadata.metadata()
}
use std::io::stderr;

use tlock_pdk::{
    runner::PluginRunner,
    tlock_api::{RpcMethod, global},
    wasmi_plugin_pdk::{rpc_message::RpcError, transport::Transport},
};
use tracing_subscriber::fmt;

async fn ping(transport: Transport, _: ()) -> Result<String, RpcError> {
    global::Ping.call_async(transport, ()).await?;
    Ok("pong".to_string())
}

fn main() {
    fmt()
        .with_writer(stderr)
        .without_time()
        .with_ansi(false)
        .compact()
        .init();

    PluginRunner::new().with_method(global::Ping, ping).run();
}
