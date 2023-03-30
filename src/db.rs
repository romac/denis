use core::fmt;

use color_eyre::owo_colors::OwoColorize;

use crate::{
    data::{Name, QClass, QType},
    trie::{DnsTrie, Key},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    A { address: [u8; 4] },
}

impl Record {
    pub fn qtype(&self) -> QType {
        match self {
            Record::A { .. } => QType::A,
        }
    }

    pub fn qclass(&self) -> QClass {
        QClass::IN
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Record::A { address } => address.to_vec(),
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
        }
    }
}

#[derive(Default)]
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

#[cfg(test)]
mod tests {
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
}
