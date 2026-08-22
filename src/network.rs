use if_addrs::{get_if_addrs, IfAddr};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

// =============================================================================
// Steam ID helpers
// =============================================================================

/// Converts a Steam2 ID into a Steam64 ID.
///
/// Steam2:
///     [U:1:55530433]
///
/// Steam64:
///     76561197960265728 + 55530433
///     = 76561198015796161
///
/// If the supplied value is already a Steam64 ID, it is returned unchanged.
///
/// If the value cannot be parsed, None is returned.

// =============================================================================
// Network helpers
// =============================================================================

/// Finds the local IPv4 address that is on the same subnet as the
/// specified server address.
///
/// This is used for the MatchZy configuration URL.
///
/// Example:
///
///     server: 192.168.178.50:27015
///     local:  192.168.178.20
///
/// returns:
///
///     Some(192.168.178.20)
pub fn log_receiver_ip(server_addr: SocketAddr) -> Option<Ipv4Addr> {
    let server_ip = match server_addr.ip() {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => return None,
    };

    let interfaces = get_if_addrs().ok()?;

    for interface in interfaces {
        let IfAddr::V4(addr) = interface.addr else {
            continue;
        };

        // Ignore loopback.
        if addr.ip.is_loopback() {
            continue;
        }

        // Check whether the server belongs to this interface's subnet.
        if same_subnet(server_ip, addr.ip, addr.netmask) {
            return Some(addr.ip);
        }
    }

    None
}

fn same_subnet(a: Ipv4Addr, b: Ipv4Addr, netmask: Ipv4Addr) -> bool {
    let a = u32::from(a);
    let b = u32::from(b);
    let mask = u32::from(netmask);

    (a & mask) == (b & mask)
}
