pub struct ClientToServerSchema
{
    pub name: &'static str,

    pub mapper_port: u16,
    pub heartbeat_period: u16,
    pub port: u16,
}

pub struct ServerToClientSchema
{
    pub name: &'static str,

    pub mapper_port: u16,
    pub heartbeat_period: u16,
}
