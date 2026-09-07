//! The LLM endpoints reachable from inside a sandbox.

use crate::registry::Endpoint;

/// The prefix naming the nginx upstream block of a backend.
const BLOCK_PREFIX: &str = "ava_";

/// The upstream endpoint listing command.
#[derive(Debug, Default)]
pub struct Upstreams {
    /// Emit an nginx map block instead of one host per line.
    pub nginx_map: bool,
}

/// Print the hosts of the registered backends, one per line or as the nginx
/// maps resolving them.
pub fn run(command: &Upstreams) -> std::io::Result<i32> {
    let registry = crate::registry::load()?;

    if command.nginx_map {
        print!("{}", nginx_map(&registry.endpoints()?));
    } else {
        for host in registry.hosts() {
            println!("{host}");
        }
    }

    Ok(0)
}

/// The nginx upstream block name of `endpoint`, for an address nginx cannot
/// resolve at request time because it stands in the hosts file of the
/// container rather than in DNS.
fn block(endpoint: &Endpoint) -> String {
    let name: String = endpoint
        .host
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();

    format!("{BLOCK_PREFIX}{name}")
}

/// Whether `endpoint` is reached at its own host name over TLS, which the
/// proxy resolves and dials directly.
fn direct(endpoint: &Endpoint) -> bool {
    endpoint.address == endpoint.host
}

/// Render the maps resolving a requested host to the upstream serving it, the
/// scheme to reach it with and the Host header it answers under, preceded by
/// an upstream block for every endpoint reached elsewhere than its host name.
///
/// Hosts absent from the maps resolve to the empty string, which the proxy
/// refuses instead of forwarding.
pub fn nginx_map(endpoints: &[Endpoint]) -> String {
    let mut rendered = String::new();

    for endpoint in endpoints.iter().filter(|endpoint| !direct(endpoint)) {
        rendered.push_str(&format!(
            "upstream {} {{\n    server {};\n}}\n\n",
            block(endpoint),
            endpoint.address
        ));
    }

    let mut map = |variable: &str, value: &dyn Fn(&Endpoint) -> String| {
        rendered.push_str(&format!("map $host ${variable} {{\n    default \"\";\n"));
        for endpoint in endpoints {
            rendered.push_str(&format!("    {} {};\n", endpoint.host, value(endpoint)));
        }
        rendered.push_str("}\n\n");
    };

    map("upstream", &|endpoint| {
        if direct(endpoint) {
            endpoint.host.clone()
        } else {
            block(endpoint)
        }
    });
    map("upstream_scheme", &|endpoint| endpoint.scheme.clone());
    map("upstream_host", &|endpoint| endpoint.address.clone());

    rendered.truncate(rendered.trim_end().len() + 1);
    rendered
}
