use color_eyre::Report;
use deku::{bitvec::BitSlice, DekuRead};
use tokio::net::UdpSocket;
use tracing::info;

const MAX_MESSAGE_SIZE: usize = 512;

use crate::data::Header;

pub async fn run(listen_addr: (&str, u16)) -> Result<(), Report> {
    let socket = UdpSocket::bind(listen_addr).await?;

    info!("Listening on {}", socket.local_addr()?);

    let mut buf = [0; MAX_MESSAGE_SIZE];
    loop {
        let (count, addr) = socket.recv_from(&mut buf).await?;
        let data = &buf[..count];

        info!("Received {count} bytes from {addr}");
        info!("Data: {data:?}");

        let (rest, message) = Header::read(BitSlice::from_slice(data), deku::ctx::Endian::Big)?;
        info!("Message: {message:#?}");
        info!("Rest: {rest:?}");
    }
}
