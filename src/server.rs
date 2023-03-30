use std::{net::SocketAddr, sync::Arc};

use color_eyre::{eyre::eyre, owo_colors::OwoColorize, Report};
use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, trace};

const MAX_MESSAGE_SIZE: usize = 512;

use crate::{
    data::{Flags, Header, Message, Question, ResourceRecord},
    db::{Db, Record},
};

pub async fn run(listen_addr: (&str, u16)) -> Result<(), Report> {
    let socket = Arc::new(UdpSocket::bind(listen_addr).await?);

    info!(
        "Listening on {}",
        socket.local_addr()?.to_string().cyan().underline(),
    );

    let mut buf = [0; MAX_MESSAGE_SIZE];
    loop {
        let (count, addr) = socket.recv_from(&mut buf).await?;
        let data = &buf[..count];

        debug!("Received {count} bytes from {addr}");
        trace!("Data: {data:?}");

        tokio::spawn(handle_request(socket.clone(), data.to_vec(), addr));
    }
}

async fn handle_request(socket: Arc<UdpSocket>, data: Vec<u8>, addr: SocketAddr) {
    let message = match Message::from_bytes((&data, 0)) {
        Ok((_, message)) => message,
        Err(err) => {
            error!("Failed to parse message: {err}");
            return;
        }
    };

    debug!("Handling message: {message:#?}");

    let response = match handle_message(message).await {
        Ok(response) => response,
        Err(err) => {
            error!("Failed to handle message: {err}");
            return;
        }
    };

    debug!("Response: {response:#?}");

    let response_data = match response.to_bytes() {
        Ok(data) => data,
        Err(err) => {
            error!("Failed to serialize response: {err}");
            return;
        }
    };

    debug!("Sending {} bytes response to {addr}", response_data.len(),);

    if let Err(err) = socket.send_to(&response_data, addr).await {
        error!("Failed to send response: {err}");
    }
}

async fn handle_message(message: Message) -> Result<Message, Report> {
    let answers = message
        .questions
        .iter()
        .map(answer_question)
        .collect::<Result<Vec<_>, _>>()?;

    let header = Header {
        id: message.header.id,
        flags: Flags::answer(message.header.flags.opcode),
        ancount: answers.len() as u16,
        qdcount: 0,
        nscount: 0,
        arcount: 0,
    };

    let response = Message {
        header,
        answers,
        questions: vec![],
        authorities: vec![],
        additionals: vec![],
    };

    Ok(response)
}

fn answer_question(question: &Question) -> Result<ResourceRecord, Report> {
    let mut db = Db::new();

    db.insert(
        &"example.com".parse().unwrap(),
        Record::A {
            address: [1, 1, 1, 1],
        },
    );

    db.insert(
        &"*.local.dev".parse().unwrap(),
        Record::A {
            address: [127, 0, 0, 1],
        },
    );

    info!(
        "<== {:<40}    {:?}",
        question.qname.blue().bold().to_string(),
        question.qtype.green().bold(),
    );

    let Some(record) = db.lookup(&question.qname, question.qtype) else {
        // TODO: Return a resource record
        return Err(eyre!("No record found"));
    };

    info!(
        "==> {:<40}    {}",
        question.qname.blue().bold().to_string(),
        record
    );

    let data = record.to_bytes();

    let answer = ResourceRecord {
        name: question.qname.clone(),
        qtype: record.qtype(),
        qclass: record.qclass(),
        ttl: 120,
        rdlength: data.len() as u16,
        data,
        options_code: None,
        options_length: None,
    };

    Ok(answer)
}
