use core::fmt;

use color_eyre::owo_colors::OwoColorize;

use crate::data::{Name, QClass, QType};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    A { address: [u8; 4] },
    CNAME { name: Name },
    TXT { text: String },
}

impl Record {
    pub fn qtype(&self) -> QType {
        match self {
            Record::A { .. } => QType::A,
            Record::CNAME { .. } => QType::CNAME,
            Record::TXT { .. } => QType::TXT,
        }
    }

    pub fn qclass(&self) -> QClass {
        QClass::IN
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Record::A { address } => address.to_vec(),
            Record::CNAME { name } => name.to_bytes(),
            Record::TXT { text } => {
                let mut bytes = vec![text.len() as u8];
                bytes.extend(text.as_bytes());
                bytes
            }
        }
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Record::A { address } => write!(
                f,
                "{:<8} {}",
                "A".green().bold(),
                format!(
                    "{}.{}.{}.{}",
                    address[0], address[1], address[2], address[3]
                )
                .yellow()
            ),
            Record::CNAME { name } => write!(f, "{:<8} {}", "CNAME".green().bold(), name),
            Record::TXT { text } => write!(f, "{:<8} {}", "TXT".green().bold(), text.italic()),
        }
    }
}
