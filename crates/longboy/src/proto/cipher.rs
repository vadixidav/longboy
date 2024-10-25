use cipher::{
    array::ArraySize,
    typenum::{U20, U8},
    Block, BlockCipherDecrypt, BlockCipherEncrypt, BlockSizeUser, KeyInit,
};
use rc5::RC5;
use typenum::{IsLess, NonZero, U256};
use zerocopy::{AsBytes, FromBytes};

use super::DatagramHeader;

pub(crate) struct Cipher<MessageSize>
{
    header_cipher: RC5<u16, U20, U8>,
    slot_cipher: RC5<u16, U20, MessageSize>,
}

impl<MessageSize> Cipher<MessageSize>
where
    MessageSize: ArraySize + IsLess<U256> + NonZero,
{
    pub(crate) fn new(key: u64) -> Self
    {
        Self {
            header_cipher: RC5::new(key.to_ne_bytes().as_ref()),
            slot_cipher: RC5::new(key.to_ne_bytes().as_ref()),
        }
    }

    pub(crate) fn encrypt_header(&self, header: &mut DatagramHeader)
    {
        self.header_cipher.encrypt_block(
            header
                .as_bytes_mut()
                .try_into()
                .expect("datagram header was a different size than expected by cipher"),
        );
    }

    pub(crate) fn decrypt_header(&self, header: &mut DatagramHeader)
    {
        self.header_cipher.decrypt_block(
            header
                .as_bytes_mut()
                .try_into()
                .expect("datagram header was a different size than expected by cipher"),
        );
    }

    pub(crate) fn encrypt_slot<T: AsBytes + FromBytes>(&self, message: &mut T)
    {
        self.slot_cipher.encrypt_block(
            message
                .as_bytes_mut()
                .try_into()
                .expect("message was a different size than expected by cipher"),
        );
    }

    pub(crate) fn decrypt_slot<T: AsBytes + FromBytes>(&self, message: &mut T)
    {
        self.slot_cipher.decrypt_block(
            message
                .as_bytes_mut()
                .try_into()
                .expect("message was a different size than expected by cipher"),
        );
    }
}
