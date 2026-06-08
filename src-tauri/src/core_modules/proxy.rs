use crate::models::types::{ProxyInfo, ProxyNode, VpnConnection};
use std::process::Command;

/// Gather all proxy/VPN information for the system
pub fn get_proxy_info() -> ProxyInfo {
    let system_proxy = get_system_proxy();
    let vpn_connections = get_vpn_connections();
    let active_client = detect_proxy_client();
    let proxy_nodes = get_proxy_nodes(&active_client);

    ProxyInfo {
        system_proxy,
        vpn_connections,
        active_client,
        proxy_nodes,
    }
}

/// Read system proxy settings via networksetup
fn get_system_proxy() -> crate::models::types::SystemProxy {
    let service = get_active_network_service();
    let service_name = service.as_deref().unwrap_or("Wi-Fi");

    let http = read_proxy_entry(service_name, "getwebproxy");
    let https = read_proxy_entry(service_name, "getsecurewebproxy");
    let socks = read_socks_proxy(service_name);
    let pac = read_pac_proxy(service_name);
    let bypass = read_bypass_domains(service_name);

    crate::models::types::SystemProxy {
        active_service: service.unwrap_or_else(|| "Unknown".to_string()),
        http,
        https,
        socks,
        pac,
        bypass,
    }
}

/// Get the primary active network service name
fn get_active_network_service() -> Option<String> {
    // Try to find the active service from -listnetworkserviceorder
    // by matching the primary interface
    let output = Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Skip the first line (header: "An asterisk (*) denotes...")
    for line in stdout.lines().skip(1) {
        let name = line.trim();
        if name.is_empty() || name.starts_with('*') {
            continue;
        }
        // Return the first non-disabled service (commonly "Wi-Fi" or "Ethernet")
        // We prefer Wi-Fi if available
        if name == "Wi-Fi" || name == "Ethernet" {
            return Some(name.to_string());
        }
    }
    // Fallback: return first available service
    stdout
        .lines()
        .skip(1)
        .find(|l| !l.trim().is_empty() && !l.starts_with('*'))
        .map(|l| l.trim().to_string())
}

/// Parse output of networksetup -getwebproxy / -getsecurewebproxy
fn read_proxy_entry(service: &str, command: &str) -> crate::models::types::ProxyEntry {
    let output = Command::new("networksetup")
        .args([format!("-{}", command), service.to_string()])
        .output();

    match output {
        Ok(ref o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            parse_proxy_output(&stdout)
        }
        _ => crate::models::types::ProxyEntry {
            enabled: false,
            server: String::new(),
            port: 0,
        },
    }
}

/// Parse "Enabled: Yes\nServer: 127.0.0.1\nPort: 7890\n..."
fn parse_proxy_output(text: &str) -> crate::models::types::ProxyEntry {
    let mut enabled = false;
    let mut server = String::new();
    let mut port = 0u16;

    for line in text.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("Enabled:") {
            enabled = val.trim().eq_ignore_ascii_case("yes");
        } else if let Some(val) = line.strip_prefix("Server:") {
            server = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("Port:") {
            port = val.trim().parse().unwrap_or(0);
        }
    }

    crate::models::types::ProxyEntry {
        enabled,
        server,
        port,
    }
}

/// Parse SOCKS proxy (different format: "Enabled: Yes\nServer: ...\nPort: ...\n...")
fn read_socks_proxy(service: &str) -> crate::models::types::ProxyEntry {
    let output = Command::new("networksetup")
        .args(["-getsocksfirewallproxy".to_string(), service.to_string()])
        .output();

    match output {
        Ok(ref o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            parse_proxy_output(&stdout)
        }
        _ => crate::models::types::ProxyEntry {
            enabled: false,
            server: String::new(),
            port: 0,
        },
    }
}

/// Parse PAC proxy URL
fn read_pac_proxy(service: &str) -> Option<String> {
    let output = Command::new("networksetup")
        .args(["-getautoproxyurl".to_string(), service.to_string()])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut url = None;
    let mut enabled = false;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("URL:") {
            let u = val.trim();
            if !u.is_empty() {
                url = Some(u.to_string());
            }
        } else if let Some(val) = line.strip_prefix("Enabled:") {
            enabled = val.trim().eq_ignore_ascii_case("yes");
        }
    }

    if enabled {
        url
    } else {
        None
    }
}

/// Read proxy bypass domains
fn read_bypass_domains(service: &str) -> Vec<String> {
    let output = Command::new("networksetup")
        .args(["-getproxybypassdomains".to_string(), service.to_string()])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty() && !l.starts_with("There aren't"))
                .collect()
        }
        _ => vec![],
    }
}

/// Detect VPN connections via scutil --nc list
fn get_vpn_connections() -> Vec<VpnConnection> {
    let output = match Command::new("scutil").arg("--nc").arg("list").output() {
        Ok(o) if o.status.success() => o,
        _ => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut connections = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Parse lines like: "* (Connected) UUID VPN (com.example.vpn) "MyVPN" [VPN:com.example.vpn]"
        // or: "  (Disconnected) UUID PPPoE (com.example.pppoe) "MyPPPoE""
        let connected = line.contains("(Connected)");

        // Extract name: find text between the last pair of quotes
        let name = extract_quoted_name(line).unwrap_or_else(|| {
            // Fallback: extract from the bundle-id pattern
            extract_bundle_name(line).unwrap_or_else(|| "Unknown VPN".to_string())
        });

        // Extract type (VPN, PPPoE, L2TP, IKEv2, etc.)
        let vpn_type = extract_vpn_type(line);

        connections.push(VpnConnection {
            name,
            connected,
            vpn_type,
        });
    }

    connections
}

fn extract_quoted_name(line: &str) -> Option<String> {
    let start = line.rfind('"')?; // last quote
    let prefix = &line[..start];
    let end = prefix.rfind('"')?;
    Some(prefix[end + 1..].to_string())
}

fn extract_bundle_name(line: &str) -> Option<String> {
    // Extract from pattern like "Name [VPN:bundle-id]"
    if let Some(bracket_start) = line.rfind('[') {
        if let Some(colon_pos) = line[bracket_start..].find(':') {
            let bundle = &line[bracket_start + colon_pos + 1..];
            let bundle = bundle.trim_end_matches(']');
            // Take last component of bundle ID
            return bundle.split('.').next_back().map(|s| s.to_string());
        }
    }
    None
}

fn extract_vpn_type(line: &str) -> String {
    if line.contains("[VPN:") {
        "VPN".to_string()
    } else if line.contains("L2TP") {
        "L2TP".to_string()
    } else if line.contains("IKEv2") {
        "IKEv2".to_string()
    } else if line.contains("IPSec") {
        "IPSec".to_string()
    } else if line.contains("PPPoE") {
        "PPPoE".to_string()
    } else if line.contains("PPP") {
        "PPP".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Detect which proxy client is running by checking known API endpoints
fn detect_proxy_client() -> Option<crate::models::types::ProxyClient> {
    // Try Clash/Mihomo API endpoints (most common)
    for port in [9090, 9097, 9093, 7891] {
        if let Some(client) = try_clash_api(port) {
            return Some(client);
        }
    }

    // Try Clash/Mihomo over a Unix socket. Clash Verge Rev (current versions)
    // disables the TCP controller (`external-controller: ''`) and exposes only
    // `external-controller-unix`, so the TCP probes above all miss it.
    if let Some(sock) = clash_unix_socket() {
        if let Some(client) = try_clash_unix(&sock) {
            return Some(client);
        }
    }

    // Try Surge API
    for port in [6171, 6170, 6166] {
        if let Some(client) = try_surge_api(port) {
            return Some(client);
        }
    }

    // Fallback: detect by process name
    detect_proxy_process()
}

/// Try to connect to Clash/Mihomo API
fn try_clash_api(port: u16) -> Option<crate::models::types::ProxyClient> {
    let url = format!("http://127.0.0.1:{}/version", port);
    let rt = tokio::runtime::Runtime::new().ok()?;
    let resp = rt.block_on(async {
        reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
    });

    let resp = resp.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = rt.block_on(resp.json()).ok()?;
    let version = body.get("version")?.as_str()?.to_string();

    // Determine if it's Clash Verge, ClashX Meta, or generic mihomo
    let client_name = identify_clash_variant(port);

    Some(crate::models::types::ProxyClient {
        name: client_name,
        client_type: "clash".to_string(),
        api_port: port,
        version: Some(version),
        mode: clash_mode_tcp(port),
        api_socket: None,
    })
}

/// Identify specific Clash variant by checking additional signals
fn identify_clash_variant(port: u16) -> String {
    // Check for Clash Verge Rev process
    if is_process_running("clash-verge") || is_process_running("Clash Verge") {
        return "Clash Verge".to_string();
    }
    if is_process_running("clashx-pro") || is_process_running("ClashX Pro") {
        return "ClashX Pro".to_string();
    }
    if is_process_running("clashx-meta") || is_process_running("ClashX Meta") {
        return "ClashX Meta".to_string();
    }
    if is_process_running("clashx") || is_process_running("ClashX") {
        return "ClashX".to_string();
    }
    // Default based on port
    match port {
        9097 => "Clash Verge".to_string(),
        _ => "Clash/mihomo".to_string(),
    }
}

/// Try to connect to Surge API
fn try_surge_api(port: u16) -> Option<crate::models::types::ProxyClient> {
    let url = format!("http://127.0.0.1:{}/v1/outbound", port);
    let rt = tokio::runtime::Runtime::new().ok()?;
    let resp = rt.block_on(async {
        reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await
    });

    let resp = resp.ok()?;
    if !resp.status().is_success() {
        return None;
    }

    let body: serde_json::Value = rt.block_on(resp.json()).ok()?;
    let mode = body
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(crate::models::types::ProxyClient {
        name: "Surge".to_string(),
        client_type: "surge".to_string(),
        api_port: port,
        version: None,
        mode,
        api_socket: None,
    })
}

/// Detect proxy client by process name (fallback)
fn detect_proxy_process() -> Option<crate::models::types::ProxyClient> {
    let clients = [
        ("clash-verge", "Clash Verge", "clash", 9097),
        ("Clash Verge", "Clash Verge", "clash", 9097),
        ("clashx-pro", "ClashX Pro", "clash", 9090),
        ("ClashX Pro", "ClashX Pro", "clash", 9090),
        ("clashx-meta", "ClashX Meta", "clash", 9090),
        ("ClashX Meta", "ClashX Meta", "clash", 9090),
        ("clashx", "ClashX", "clash", 9090),
        ("ClashX", "ClashX", "clash", 9090),
        ("mihomo", "mihomo", "clash", 9090),
        ("surge", "Surge", "surge", 6171),
        ("Surge", "Surge", "surge", 6171),
        ("v2ray", "V2Ray", "v2ray", 0),
        ("V2Ray", "V2Ray", "v2ray", 0),
        ("xray", "Xray", "xray", 0),
        ("sing-box", "sing-box", "sing-box", 0),
        ("ss-local", "Shadowsocks", "ss", 0),
    ];

    for (process, name, client_type, port) in &clients {
        if is_process_running(process) {
            return Some(crate::models::types::ProxyClient {
                name: name.to_string(),
                client_type: client_type.to_string(),
                api_port: *port,
                version: None,
                mode: None,
                api_socket: None,
            });
        }
    }

    None
}

fn is_process_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", name])
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Get proxy nodes from the active client's API
fn get_proxy_nodes(client: &Option<crate::models::types::ProxyClient>) -> Vec<ProxyNode> {
    let client = match client {
        Some(c) => c,
        None => return vec![],
    };

    match client.client_type.as_str() {
        "clash" => match &client.api_socket {
            Some(sock) => get_clash_nodes_unix(sock),
            None => get_clash_nodes(client.api_port),
        },
        "surge" => get_surge_nodes(client.api_port),
        _ => vec![],
    }
}

/// Parse the `-ext-ctl-unix <path>` token out of a `ps` args dump (pure).
fn parse_ext_ctl_unix(args_text: &str) -> Option<String> {
    for line in args_text.lines() {
        if let Some(idx) = line.find("-ext-ctl-unix") {
            let rest = &line[idx + "-ext-ctl-unix".len()..];
            if let Some(tok) = rest.split_whitespace().next() {
                if !tok.is_empty() {
                    return Some(tok.to_string());
                }
            }
        }
    }
    None
}

/// Discover the Clash/Mihomo external-controller Unix socket path.
fn clash_unix_socket() -> Option<String> {
    // Preferred: read the live mihomo process's `-ext-ctl-unix <path>` argument
    // (works regardless of where the config lives or what path was chosen).
    if let Ok(out) = Command::new("ps").args(["-axww", "-o", "args="]).output() {
        let text = String::from_utf8_lossy(&out.stdout);
        if let Some(path) = parse_ext_ctl_unix(&text) {
            if std::path::Path::new(&path).exists() {
                return Some(path);
            }
        }
    }
    // Fallback: Clash Verge Rev's default socket location.
    let default = "/tmp/verge/verge-mihomo.sock";
    if std::path::Path::new(default).exists() {
        return Some(default.to_string());
    }
    None
}

/// GET a JSON document from a Clash/Mihomo API exposed over a Unix socket.
/// Uses `curl --unix-socket` (ships with macOS) — consistent with the other
/// system shell-outs and avoids a chunked-encoding hand-roll. The Unix socket
/// is local-trusted by mihomo, so no `secret` is required.
fn curl_unix_json(socket: &str, path: &str) -> Option<serde_json::Value> {
    let url = format!("http://localhost{}", path);
    let out = Command::new("curl")
        .args(["-s", "-m", "2", "--unix-socket", socket, &url])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

/// Probe a Clash/Mihomo controller over a Unix socket.
fn try_clash_unix(socket: &str) -> Option<crate::models::types::ProxyClient> {
    let body = curl_unix_json(socket, "/version")?;
    let version = body.get("version")?.as_str()?.to_string();
    let mode = curl_unix_json(socket, "/configs").and_then(|c| {
        c.get("mode")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
    });
    Some(crate::models::types::ProxyClient {
        name: identify_clash_variant(0),
        client_type: "clash".to_string(),
        api_port: 0,
        version: Some(version),
        mode,
        api_socket: Some(socket.to_string()),
    })
}

/// Fetch the current Clash mode (rule/global/direct) from a TCP controller.
fn clash_mode_tcp(port: u16) -> Option<String> {
    let url = format!("http://127.0.0.1:{}/configs", port);
    let rt = tokio::runtime::Runtime::new().ok()?;
    let resp = rt
        .block_on(async {
            reqwest::Client::new()
                .get(&url)
                .timeout(std::time::Duration::from_millis(500))
                .send()
                .await
        })
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = rt.block_on(resp.json()).ok()?;
    body.get("mode")
        .and_then(|m| m.as_str())
        .map(|s| s.to_string())
}

/// Fetch proxy nodes from a Clash/Mihomo API exposed over a Unix socket.
fn get_clash_nodes_unix(socket: &str) -> Vec<ProxyNode> {
    match curl_unix_json(socket, "/proxies") {
        Some(body) => parse_clash_proxies(&body),
        None => vec![],
    }
}

/// Parse the `/proxies` response into selector/test/fallback group nodes.
fn parse_clash_proxies(body: &serde_json::Value) -> Vec<ProxyNode> {
    let proxies = match body.get("proxies").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return vec![],
    };

    let mut nodes = Vec::new();

    for (_name, proxy) in proxies {
        let proxy_type = proxy.get("type").and_then(|t| t.as_str()).unwrap_or("");

        // Only show selector/url-test/fallback groups (not individual proxies)
        let is_group = matches!(
            proxy_type,
            "Selector" | "URLTest" | "Fallback" | "LoadBalance"
        );

        if !is_group {
            continue;
        }

        let group_name = proxy
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        let now = proxy
            .get("now")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        // Get latency from history
        let delay = proxy
            .get("history")
            .and_then(|h| h.as_array())
            .and_then(|arr| arr.last())
            .and_then(|h| h.get("delay"))
            .and_then(|d| d.as_u64());

        let all = proxy
            .get("all")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        nodes.push(ProxyNode {
            name: group_name,
            selected: now,
            delay,
            node_type: proxy_type.to_string(),
            available_nodes: all,
        });
    }

    nodes
}

/// Fetch proxy nodes from Clash/Mihomo API
fn get_clash_nodes(port: u16) -> Vec<ProxyNode> {
    let url = format!("http://127.0.0.1:{}/proxies", port);
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let resp = rt.block_on(async {
        reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
    });

    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        _ => return vec![],
    };

    let body: serde_json::Value = match rt.block_on(resp.json()) {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    parse_clash_proxies(&body)
}

/// Fetch proxy nodes from Surge API
fn get_surge_nodes(port: u16) -> Vec<ProxyNode> {
    let url = format!("http://127.0.0.1:{}/v1/policy_groups", port);
    let rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    let resp = rt.block_on(async {
        reqwest::Client::new()
            .get(&url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
    });

    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        _ => return vec![],
    };

    let body: serde_json::Value = match rt.block_on(resp.json()) {
        Ok(b) => b,
        Err(_) => return vec![],
    };

    let groups = match body.get("policy_groups").and_then(|g| g.as_array()) {
        Some(g) => g,
        None => return vec![],
    };

    let mut nodes = Vec::new();

    for group in groups {
        let name = group
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        let selected = group
            .get("selected")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let delay = group.get("latency").and_then(|l| l.as_u64());

        let type_str = group
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();

        let all = group
            .get("candidates")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        nodes.push(ProxyNode {
            name,
            selected,
            delay,
            node_type: type_str,
            available_nodes: all,
        });
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy_output() {
        let text = "Enabled: Yes\nServer: 127.0.0.1\nPort: 7890\nAuthenticated Proxy Enabled: 0\n";
        let entry = parse_proxy_output(text);
        assert!(entry.enabled);
        assert_eq!(entry.server, "127.0.0.1");
        assert_eq!(entry.port, 7890);
    }

    #[test]
    fn test_parse_proxy_output_disabled() {
        let text = "Enabled: No\nServer: \nPort: 0\nAuthenticated Proxy Enabled: 0\n";
        let entry = parse_proxy_output(text);
        assert!(!entry.enabled);
        assert_eq!(entry.server, "");
        assert_eq!(entry.port, 0);
    }

    #[test]
    fn test_extract_vpn_type() {
        assert_eq!(extract_vpn_type("[VPN:com.example.vpn]"), "VPN");
        assert_eq!(extract_vpn_type("L2TP connection"), "L2TP");
        assert_eq!(extract_vpn_type("IKEv2 tunnel"), "IKEv2");
        assert_eq!(extract_vpn_type("PPPoE link"), "PPPoE");
        assert_eq!(extract_vpn_type("random text"), "Unknown");
    }

    #[test]
    fn test_extract_quoted_name() {
        assert_eq!(
            extract_quoted_name("* (Connected) UUID VPN (com.test) \"MyVPN\" [VPN:com.test]"),
            Some("MyVPN".to_string())
        );
        assert_eq!(extract_quoted_name("no quotes here"), None);
    }

    #[test]
    fn test_get_active_network_service() {
        let service = get_active_network_service();
        // Should find some network service
        assert!(service.is_some(), "should detect an active network service");
    }

    #[test]
    fn test_get_system_proxy() {
        let proxy = get_system_proxy();
        // Should have a service name
        assert!(
            !proxy.active_service.is_empty(),
            "should have an active service name"
        );
    }

    #[test]
    fn test_detect_proxy_client() {
        // This test is non-deterministic (depends on what's running)
        // Just verify it doesn't panic
        let _client = detect_proxy_client();
    }

    #[test]
    fn test_get_vpn_connections() {
        // Just verify it doesn't panic
        let _vpns = get_vpn_connections();
    }

    #[test]
    fn test_parse_ext_ctl_unix() {
        let args = "/Applications/Clash Verge.app/Contents/MacOS/verge-mihomo -d /Users/x/Library/Application Support/io.github.clash-verge-rev.clash-verge-rev -f /Users/x/.../clash-verge.yaml -ext-ctl-unix /tmp/verge/verge-mihomo.sock\n/usr/sbin/cfprefsd";
        assert_eq!(
            parse_ext_ctl_unix(args),
            Some("/tmp/verge/verge-mihomo.sock".to_string())
        );
        assert_eq!(parse_ext_ctl_unix("no socket here\nfoo bar"), None);
    }

    #[test]
    fn test_parse_clash_proxies() {
        // Only Selector/URLTest/Fallback/LoadBalance become nodes; plain proxies
        // are skipped.
        let body: serde_json::Value = serde_json::from_str(
            r#"{"proxies":{
                "Auto":{"type":"URLTest","name":"Auto","now":"HK","all":["HK","JP"],
                        "history":[{"delay":42}]},
                "HK":{"type":"Shadowsocks","name":"HK"},
                "Proxy":{"type":"Selector","name":"Proxy","now":"Auto","all":["Auto","HK"],"history":[]}
            }}"#,
        )
        .unwrap();
        let mut nodes = parse_clash_proxies(&body);
        nodes.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(nodes.len(), 2, "two groups, the plain SS proxy excluded");
        let auto = nodes.iter().find(|n| n.name == "Auto").unwrap();
        assert_eq!(auto.selected, "HK");
        assert_eq!(auto.delay, Some(42));
        assert_eq!(auto.node_type, "URLTest");
        assert_eq!(auto.available_nodes, vec!["HK", "JP"]);
    }

    #[test]
    fn test_clash_unix_socket_no_panic() {
        // May or may not find a socket depending on the machine; must not panic.
        let _ = clash_unix_socket();
    }
}
