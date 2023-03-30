use color_eyre::Report;
use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::net::UdpSocket;
use tracing::info;

const MAX_MESSAGE_SIZE: usize = 512;

use crate::data::{Flags, Header, Message, RCode, ResourceRecord};

pub async fn run(listen_addr: (&str, u16)) -> Result<(), Report> {
    let socket = UdpSocket::bind(listen_addr).await?;

    info!("Listening on {}", socket.local_addr()?);

    let mut buf = [0; MAX_MESSAGE_SIZE];
    loop {
        let (count, addr) = socket.recv_from(&mut buf).await?;
        let data = &buf[..count];

        info!("Received {count} bytes from {addr}");
        info!("Data: {data:?}");

        let (_rest, message) = Message::from_bytes((data, 0))?;
        info!("Message: {message:#?}");

        let question = &message.questions[0];

        let answer = ResourceRecord {
            name: question.qname.clone(),
            r#type: crate::data::QType::A,
            qclass: crate::data::QClass::IN,
            ttl: 1024,
            rdlength: 4,
            data: vec![127, 0, 0, 1],
            options_code: None,
            options_length: None,
        };

        let response = Message {
            header: Header {
                id: message.header.id,
                flags: Flags {
                    qr: true,
                    opcode: message.header.flags.opcode,
                    aa: true,
                    tc: false,
                    rd: false,
                    ra: false,
                    z: 0,
                    rcode: RCode::NoError,
                },
                qdcount: 0,
                ancount: 1,
                nscount: 0,
                arcount: 0,
            },
            questions: vec![],
            answers: vec![answer],
            authorities: vec![],
            additionals: vec![],
        };

        info!("Response: {response:#?}");
        let bytes = response.to_bytes()?;
        info!("Response bytes: {bytes:?}");

        socket.send_to(&bytes, addr).await?;
    }
}
