use std::net::UdpSocket;

use anyhow::Result;

pub(crate) trait UdpSocketExt
{
    fn set_qos_audio_video(&self) -> Result<()>;
    fn set_qos_background(&self) -> Result<()>;
    fn set_qos_voice(&self) -> Result<()>;
}

impl UdpSocketExt for UdpSocket
{
    fn set_qos_audio_video(&self) -> Result<()>
    {
        Ok(())
    }

    fn set_qos_background(&self) -> Result<()>
    {
        Ok(())
    }

    fn set_qos_voice(&self) -> Result<()>
    {
        Ok(())
    }
}
