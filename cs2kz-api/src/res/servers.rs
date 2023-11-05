use {super::PlayerInfo, serde::Serialize, std::net::Ipv4Addr, utoipa::ToSchema};

#[derive(Debug, Serialize, ToSchema)]
pub struct Server {
	pub id: u16,
	pub name: String,
	pub owned_by: PlayerInfo,

	#[schema(value_type = String)]
	pub ip: Ipv4Addr,

	pub port: u16,
}
