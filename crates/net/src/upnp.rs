//! UPnP Port Mapping for LAN/Internet multiplayer.
//!
//! Provides automatic port forwarding via UPnP/IGD for hosting LAN games
//! accessible over the internet.
//!
//! Uses the oxy-upnp-igd crate which provides IGD (Internet Gateway Device)
//! port mapping functionality with the smol async runtime.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use tracing::{debug, error, info, warn};

use oxy_upnp_igd::{Gateway, Protocol as OxyProtocol, discover_gateways};
use smol::Executor;

/// Result type for UPnP operations.
pub type UpnpResult<T> = Result<T, UpnpError>;

/// Errors that can occur during UPnP operations.
#[derive(Debug, thiserror::Error)]
pub enum UpnpError {
    #[error("UPnP discovery failed: {0}")]
    DiscoveryFailed(String),

    #[error("No UPnP gateway found")]
    NoGateway,

    #[error("Failed to add port mapping: {0}")]
    AddPortMappingFailed(String),

    #[error("Failed to remove port mapping: {0}")]
    RemovePortMappingFailed(String),

    #[error("Failed to get external IP: {0}")]
    GetExternalIpFailed(String),

    #[error("Invalid port: {0}")]
    InvalidPort(u16),

    #[error("UPnP operation timeout")]
    Timeout,

    #[error("UPnP not available: {0}")]
    NotAvailable(String),
}

/// Protocol for port mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortMappingProtocol {
    TCP,
    UDP,
}

/// UPnP port manager for automatic port forwarding.
pub struct UpnpPortManager {
    gateway: Option<oxy_upnp_igd::Gateway>,
    external_ip: Option<std::net::IpAddr>,
    mapped_ports: Vec<u16>,
    /// Whether UPnP is available on this network
    available: bool,
}

impl UpnpPortManager {
    /// Create a new UPnP port manager.
    pub fn new() -> Self {
        Self {
            gateway: None,
            external_ip: None,
            mapped_ports: Vec::new(),
            available: false,
        }
    }

    /// Discover the UPnP gateway on the network.
    pub fn discover(&mut self, timeout: Duration) -> UpnpResult<()> {
        debug!("Discovering UPnP gateway (timeout: {:?})...", timeout);

        // Run async discovery in a blocking context using smol
        let result: Result<(Gateway, Ipv4Addr), UpnpError> = smol::block_on(async {
            let gateways = oxy_upnp_igd::discover_gateways(timeout)
                .await
                .map_err(|e| UpnpError::DiscoveryFailed(e.to_string()))?;

            if let Some(gateway) = gateways.first().cloned() {
                let external_ip = gateway
                    .get_external_ip()
                    .await
                    .map_err(|e| UpnpError::GetExternalIpFailed(e.to_string()))?;

                info!("UPnP gateway discovered: external IP: {}", external_ip);
                Ok((gateway, external_ip))
            } else {
                Err(UpnpError::NoGateway)
            }
        });

        match result {
            Ok((gateway, external_ip)) => {
                self.gateway = Some(gateway);
                self.external_ip = Some(std::net::IpAddr::V4(external_ip));
                self.available = true;
                info!("UPnP discovery completed. Available: true");
                Ok(())
            }
            Err(e) => {
                self.available = false;
                info!("UPnP discovery completed. Available: false ({})", e);
                Err(e)
            }
        }
    }

    /// Get the external IP address of the gateway.
    pub fn external_ip(&self) -> Option<std::net::IpAddr> {
        self.external_ip
    }

    /// Check if UPnP gateway is available.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Add a port mapping for a local server.
    ///
    /// This forwards `external_port` on the gateway to `local_port` on the local machine.
    /// If `external_port` is 0, the same port will be used.
    pub fn add_port_mapping(
        &mut self,
        local_port: u16,
        external_port: u16,
        protocol: PortMappingProtocol,
        description: &str,
    ) -> UpnpResult<u16> {
        if !self.available {
            return Err(UpnpError::NotAvailable(
                "UPnP gateway not discovered".to_string(),
            ));
        }

        if local_port == 0 {
            return Err(UpnpError::InvalidPort(local_port));
        }

        let gateway = self
            .gateway
            .as_ref()
            .ok_or(UpnpError::NotAvailable("No gateway".to_string()))?;
        let port = if external_port == 0 {
            local_port
        } else {
            external_port
        };
        let oxy_protocol = match protocol {
            PortMappingProtocol::TCP => oxy_upnp_igd::Protocol::TCP,
            PortMappingProtocol::UDP => oxy_upnp_igd::Protocol::UDP,
        };

        debug!(
            "Adding UPnP port mapping: external={}, local={}, protocol={:?}",
            port, local_port, protocol
        );

        // Get local IP address for the gateway
        let local_addr = get_local_ipv4_for_gateway(gateway)
            .map_err(|e| UpnpError::AddPortMappingFailed(e.to_string()))?;

        // Run async add_port_mapping in blocking context using smol
        let result: Result<(), UpnpError> = smol::block_on(async {
            gateway
                .add_port_mapping(
                    local_port,
                    port,
                    local_addr,
                    oxy_protocol,
                    description,
                    3600,
                )
                .await
                .map_err(|e| UpnpError::AddPortMappingFailed(e.to_string()))
        });

        result?;

        info!(
            "UPnP port mapping added: {} -> {} ({:?})",
            port, local_port, protocol
        );

        self.mapped_ports.push(port);
        Ok(port)
    }

    /// Remove a port mapping.
    pub fn remove_port_mapping(
        &mut self,
        external_port: u16,
        protocol: PortMappingProtocol,
    ) -> UpnpResult<()> {
        if !self.available {
            return Err(UpnpError::NotAvailable(
                "UPnP gateway not discovered".to_string(),
            ));
        }

        let gateway = self
            .gateway
            .as_ref()
            .ok_or(UpnpError::NotAvailable("No gateway".to_string()))?;
        let oxy_protocol = match protocol {
            PortMappingProtocol::TCP => oxy_upnp_igd::Protocol::TCP,
            PortMappingProtocol::UDP => oxy_upnp_igd::Protocol::UDP,
        };

        debug!(
            "Removing UPnP port mapping: external={}, protocol={:?}",
            external_port, protocol
        );

        // Run async delete_port_mapping in blocking context
        let result: Result<(), UpnpError> = smol::block_on(async {
            gateway
                .delete_port_mapping(external_port, oxy_protocol)
                .await
                .map_err(|e| UpnpError::RemovePortMappingFailed(e.to_string()))
        });

        result?;

        info!(
            "UPnP port mapping removed: {} ({:?})",
            external_port, protocol
        );

        self.mapped_ports.retain(|&p| p != external_port);
        Ok(())
    }

    /// Get all currently mapped ports.
    pub fn mapped_ports(&self) -> &[u16] {
        &self.mapped_ports
    }

    /// Clean up all port mappings.
    pub fn cleanup_all(&mut self) {
        if !self.available {
            return;
        }

        if let Some(gateway) = &self.gateway {
            let ports = self.mapped_ports.clone();

            for &port in &ports {
                // Try both TCP and UDP
                let _ = smol::block_on(async {
                    let _ = gateway
                        .delete_port_mapping(port, oxy_upnp_igd::Protocol::TCP)
                        .await;
                });

                let _ = smol::block_on(async {
                    let _ = gateway
                        .delete_port_mapping(port, oxy_upnp_igd::Protocol::UDP)
                        .await;
                });
            }
        }
        self.mapped_ports.clear();
    }
}

impl Default for UpnpPortManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for UpnpPortManager {
    fn drop(&mut self) {
        self.cleanup_all();
    }
}

/// Helper to create a UPnP port manager and map the default MCLRE port (25565).
pub fn setup_default_port_mapping(local_port: u16) -> UpnpResult<UpnpPortManager> {
    let mut manager = UpnpPortManager::new();

    // Try to discover gateway
    if let Err(e) = manager.discover(Duration::from_secs(5)) {
        warn!("UPnP discovery failed: {}. Port mapping unavailable.", e);
        return Ok(manager); // Return manager even if no gateway (local only)
    }

    // Map both TCP and UDP for the game port
    let _ = manager.add_port_mapping(
        local_port,
        local_port,
        PortMappingProtocol::TCP,
        "MCLRE Game Server TCP",
    );
    let _ = manager.add_port_mapping(
        local_port,
        local_port,
        PortMappingProtocol::UDP,
        "MCLRE Game Server UDP",
    );

    Ok(manager)
}

/// Helper to get the public address for sharing with friends.
/// Returns the external IP and port if UPnP is available.
pub fn get_public_address(manager: &UpnpPortManager, local_port: u16) -> Option<SocketAddr> {
    manager
        .external_ip()
        .and_then(|ip| Some(SocketAddr::new(ip, local_port)))
}

/// Helper to get the local IPv4 address that can reach the given gateway.
/// This is needed for the AddPortMapping SOAP request.
fn get_local_ipv4_for_gateway(gateway: &oxy_upnp_igd::Gateway) -> Result<Ipv4Addr, String> {
    smol::block_on(async {
        oxy_upnp_igd::get_local_ipv4_for_gateway(gateway)
            .await
            .map_err(|e| e.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upnp_manager_creation() {
        let manager = UpnpPortManager::new();
        assert!(!manager.is_available());
        assert_eq!(manager.mapped_ports(), &[] as &[u16]);
    }

    #[test]
    fn test_port_mapping_protocol() {
        assert_eq!(PortMappingProtocol::TCP, PortMappingProtocol::TCP);
        assert_eq!(PortMappingProtocol::UDP, PortMappingProtocol::UDP);
        assert_ne!(PortMappingProtocol::TCP, PortMappingProtocol::UDP);
    }
}
