use std::{net::SocketAddr, path::Path, sync::Arc, time::Instant};

use color_eyre::{owo_colors::OwoColorize, Report};
use deku::{DekuContainerRead, DekuContainerWrite};
use tokio::net::UdpSocket;
use tracing::{debug, error, info, trace};

const MAX_MESSAGE_SIZE: usize = 512;

use crate::{
    data::{Flags, Header, Message, Question, ResourceRecord},
    db::Db,
};

#[derive(Clone, Debug)]
struct Forwarder {
    socket: Arc<UdpSocket>,
}

impl Forwarder {
    pub async fn connect(addr: SocketAddr) -> Result<Self, Report> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
        socket.connect(addr).await?;

        info!(
            "Connected to upstream at {}",
            addr.to_string().cyan().underline(),
        );

        Ok(Self { socket })
    }

    pub async fn forward(&self, data: &[u8]) -> Result<Vec<u8>, Report> {
        self.socket.send(data).await?;

        let mut buf = [0; MAX_MESSAGE_SIZE];
        let count = self.socket.recv(&mut buf).await?;

        Ok(buf[..count].to_vec())
    }
}

pub async fn run(
    db: &Path,
    listen_addr: (&str, u16),
    upstream_addr: SocketAddr,
) -> Result<(), Report> {
    let db = Arc::new(crate::db::load(db)?);
    let socket = Arc::new(UdpSocket::bind(listen_addr).await?);
    let forwarder = Forwarder::connect(upstream_addr).await?;

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
            forwarder.clone(),
            socket.clone(),
            data.to_vec(),
            addr,
        ));
    }
}

async fn forward(forwarder: &Forwarder, data: &[u8]) -> Result<Message, Report> {
    let data = forwarder.forward(data).await?;
    trace!("Data received from upstream: {data:?}");

    let (_, msg) = Message::from_bytes((&data, 0))?;
    Ok(msg)
}

async fn handle_request(
    db: Arc<Db>,
    forwarder: Forwarder,
    socket: Arc<UdpSocket>,
    data: Vec<u8>,
    addr: SocketAddr,
) {
    let message = match Message::from_bytes((&data, 0)) {
        Ok((_, message)) => message,
        Err(err) => {
            error!("Failed to parse message: {err}");
            return;
        }
    };

    debug!("Handling message: {message:#?}");

    let response = match handle_message(&db, &message).await {
        Ok(Some(response)) => response,
        Ok(None) => {
            debug!("Forwarding request to upstream");

            match forward(&forwarder, &data).await {
                Ok(response) => response,
                Err(err) => {
                    error!("Failed to forward request: {err}");
                    return;
                }
            }
        }
        Err(err) => {
            error!("Failed to handle message: {err}");
            return;
        }
    };

    // debug!("Response: {response:#?}");

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

async fn handle_message(db: &Db, message: &Message) -> Result<Option<Message>, Report> {
    let answers = message
        .questions
        .iter()
        .map(|q| answer_question(db, q))
        .collect::<Result<Option<Vec<_>>, _>>()?;

    let Some(answers) = answers else {
        return Ok(None);
    };

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

    Ok(Some(response))
}

fn answer_question(db: &Db, question: &Question) -> Result<Option<ResourceRecord>, Report> {
    let now = Instant::now();

    info!(
        "<== {:<50}    {:?}",
        question.qname.blue().bold().to_string(),
        question.qtype.green().bold(),
    );

    let record = db.lookup(&question.qname, question.qtype);

    let Some(record) = record else {
            return Ok(None);
        };

    let data = record.to_bytes();

    let answer = ResourceRecord {
        name: question.qname.clone(),
        qtype: record.qtype(),
        qclass: record.qclass(),
        ttl: 1,
        rdlength: data.len() as u16,
        data,
        options_code: None,
        options_length: None,
    };

    let elapsed = now.elapsed().as_millis();

    info!(
        "==> {:<50}    {:#}          {}",
        question.qname.blue().bold().to_string(),
        record,
        format!("{elapsed}ms").dimmed()
    );

    Ok(Some(answer))
}
