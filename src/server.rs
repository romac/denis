use std::{net::SocketAddr, path::Path, sync::Arc};

use color_eyre::{owo_colors::OwoColorize, Report};
use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, trace};

const MAX_MESSAGE_SIZE: usize = 512;

use crate::{
    data::{Flags, Header, Message, Question, ResourceRecord},
    db::{Db, Record},
};

pub async fn run(db: &Path, listen_addr: (&str, u16)) -> Result<(), Report> {
    let db = Arc::new(crate::db::load(db)?);

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

        tokio::spawn(handle_request(
            db.clone(),
            socket.clone(),
            data.to_vec(),
            addr,
        ));
    }
}

async fn handle_request(db: Arc<Db>, socket: Arc<UdpSocket>, data: Vec<u8>, addr: SocketAddr) {
    let message = match Message::from_bytes((&data, 0)) {
        Ok((_, message)) => message,
        Err(err) => {
            error!("Failed to parse message: {err}");
            return;
        }
    };

    debug!("Handling message: {message:#?}");

    let response = match handle_message(&db, message).await {
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

async fn handle_message(db: &Db, message: Message) -> Result<Message, Report> {
    let answers = message
        .questions
        .iter()
        .map(|q| answer_question(db, q))
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

fn answer_question(db: &Db, question: &Question) -> Result<ResourceRecord, Report> {
    let no_such_domain = Record::TXT {
        text: "No such domain".to_string(),
    };

    info!(
        "<== {:<40}    {:?}",
        question.qname.blue().bold().to_string(),
        question.qtype.green().bold(),
    );

    let record = db
        .lookup(&question.qname, question.qtype)
        .unwrap_or(&no_such_domain);

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
