//! The LLM endpoints reachable from inside a sandbox.

/// The upstream endpoint listing command.
#[derive(Debug, Default)]
pub struct Upstreams {
    /// Emit an nginx map block instead of one host per line.
    pub nginx_map: bool,
}

/// The allowed upstream hosts.
///
/// This is the single source for the nginx allowlist and the sandbox host
/// entries.
pub const UPSTREAMS: [&str; 2] = ["llm.substrate.dev", "api.anthropic.com"];

/// Render the map resolving a requested host to the upstream serving it.
///
/// Hosts absent from the block map to the empty string, which the proxy
/// refuses instead of forwarding.
pub fn nginx_map() -> String {
    let mut block = String::from("map $host $upstream {\n    default \"\";\n");

    for host in UPSTREAMS {
        block.push_str(&format!("    {host} {host};\n"));
    }

    block.push_str("}\n");
    block
}
