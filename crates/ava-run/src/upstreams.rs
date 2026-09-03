//! The LLM endpoints reachable from inside a sandbox.

/// The upstream endpoint listing command.
#[derive(Debug, Default)]
pub struct Upstreams {
    /// Emit an nginx map block instead of one host per line.
    pub nginx_map: bool,
}

/// Print the hosts of the registered backends, one per line or as the nginx
/// map resolving them.
pub fn run(command: &Upstreams) -> std::io::Result<i32> {
    let hosts = crate::registry::load()?.hosts();

    if command.nginx_map {
        print!("{}", nginx_map(&hosts));
    } else {
        for host in hosts {
            println!("{host}");
        }
    }

    Ok(0)
}

/// Render the map resolving a requested host to the upstream serving it.
///
/// Hosts absent from the block map to the empty string, which the proxy
/// refuses instead of forwarding.
pub fn nginx_map(hosts: &[String]) -> String {
    let mut block = String::from("map $host $upstream {\n    default \"\";\n");

    for host in hosts {
        block.push_str(&format!("    {host} {host};\n"));
    }

    block.push_str("}\n");
    block
}
