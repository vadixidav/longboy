use std::net::IpAddr;

pub trait Session
{
    fn ip_addr(&self) -> IpAddr;
    fn session_id(&self) -> u64;
    fn cipher_key(&self) -> u64;
}
