use std::net::{IpAddr, Ipv6Addr, SocketAddr};

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct ServerConfig {
    #[serde(default = "default_ip_addr")]
    pub ip_addr: IpAddr,

    #[serde(default = "default_port")]
    pub port: u16,
}

impl ServerConfig {
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip_addr, self.port)
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip_addr: default_ip_addr(),
            port: default_port(),
        }
    }
}

fn default_ip_addr() -> IpAddr {
    IpAddr::V6(Ipv6Addr::UNSPECIFIED)
}

fn default_port() -> u16 {
    0
}
