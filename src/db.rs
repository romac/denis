use core::fmt;
use std::{path::Path, str::FromStr};

use color_eyre::{eyre::eyre, owo_colors::OwoColorize, Report};

use crate::{
    data::{Name, QClass, QType},
    trie::{DnsTrie, Key},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    A { address: [u8; 4] },
    CNAME { name: Name },
}

impl Record {
    pub fn qtype(&self) -> QType {
        match self {
            Record::A { .. } => QType::A,
            Record::CNAME { .. } => QType::CNAME,
        }
    }

    pub fn qclass(&self) -> QClass {
        QClass::IN
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Record::A { address } => address.to_vec(),
            Record::CNAME { name } => name.to_string().into_bytes(),
        }
    }
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Record::A { address } => write!(
                f,
                "{}    {}",
                "A".green().bold(),
                format!(
                    "{}.{}.{}.{}",
                    address[0], address[1], address[2], address[3]
                )
                .yellow()
            ),
            Record::CNAME { name } => write!(f, "{}    {}", "CNAME".green().bold(), name),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Db {
    trie: DnsTrie<Record>,
}

impl Db {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &Name, record: Record) {
        let key = &name
            .labels()
            .map(|label| {
                if label == "*" {
                    Key::Wildcard
                } else {
                    Key::Label(label.to_owned())
                }
            })
            .rev()
            .collect::<Vec<_>>();

        self.trie.insert(key, record);
    }

    pub fn lookup(&self, name: &Name, qtype: QType) -> Option<&Record> {
        let key = &name
            .labels()
            .map(|label| Key::Label(label.to_owned()))
            .rev()
            .collect::<Vec<_>>();

        self.trie
            .lookup(key)
            .filter(|record| qtype == QType::ANY || record.qtype() == qtype)
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<Db, Report> {
    use std::fs::File;

    let file = File::open(path)?;
    from_reader(file)
}

pub fn from_reader(reader: impl std::io::Read) -> Result<Db, Report> {
    use std::io::{BufRead, BufReader};

    let mut db = Db::new();

    let reader = BufReader::new(reader);
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let (name, record) = parse_line(line)?;
        db.insert(&name, record);
    }

    Ok(db)
}

fn parse_line(line: &str) -> Result<(Name, Record), Report> {
    let mut parts = line.split_whitespace();

    let name = parts.next().unwrap();
    let qtype = parts.next().unwrap();
    let data = parts.next().unwrap();

    let name = Name::new(name);
    let qtype = QType::from_str(qtype)?;

    let record = match qtype {
        QType::A => Record::A {
            address: parse_ip(data)?,
        },
        QType::CNAME => Record::CNAME {
            name: Name::new(data),
        },
        other => return Err(eyre!("unsupported record type: {}", other)),
    };

    Ok((name, record))
}

fn parse_ip(ip: &str) -> Result<[u8; 4], Report> {
    let mut parts = ip.split('.');

    let address = [
        parts.next().unwrap().parse()?,
        parts.next().unwrap().parse()?,
        parts.next().unwrap().parse()?,
        parts.next().unwrap().parse()?,
    ];

    Ok(address)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn normal() {
        let mut db = Db::new();

        let name = Name::new("example.com");
        let record = Record::A {
            address: [1, 1, 1, 1],
        };

        db.insert(&name, record.clone());

        assert_eq!(db.lookup(&name, QType::A), Some(&record));
    }

    #[test]
    fn normal_wrong_class() {
        let mut db = Db::new();

        let name = Name::new("example.com");
        let record = Record::A {
            address: [1, 1, 1, 1],
        };

        db.insert(&name, record);

        assert_eq!(db.lookup(&name, QType::CNAME), None);
    }

    #[test]
    fn wildcard() {
        let mut db = Db::new();

        let record = Record::A {
            address: [127, 0, 0, 1],
        };

        db.insert(&Name::new("*.local.dev"), record.clone());

        assert_eq!(
            db.lookup(&Name::new("denis.local.dev"), QType::A),
            Some(&record)
        );
    }

    #[test]
    fn wildcard_wrong_class() {
        let mut db = Db::new();

        let record = Record::A {
            address: [127, 0, 0, 1],
        };

        db.insert(&Name::new("*.local.dev"), record);

        assert_eq!(db.lookup(&Name::new("denis.local.dev"), QType::CNAME), None);
    }

    #[test]
    fn parse_db() {
        let content = r#"
            # Example domain
            example.com    CNAME    www.example.com

            # Local domains
            *.local.dev    A        127.0.0.1
            "#;

        let db = from_reader(Cursor::new(content)).unwrap();
        dbg!(&db);

        assert_eq!(
            db.lookup(&Name::new("example.com"), QType::CNAME),
            Some(&Record::CNAME {
                name: Name::new("www.example.com"),
            })
        );

        assert_eq!(
            db.lookup(&Name::new("denis.local.dev"), QType::A),
            Some(&Record::A {
                address: [127, 0, 0, 1],
            })
        );
    }
}
