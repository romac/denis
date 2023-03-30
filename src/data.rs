#![allow(clippy::upper_case_acronyms)]

use core::fmt;

use deku::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Message {
    pub header: Header,
    #[deku(count = "header.qdcount")]
    pub questions: Vec<Question>,
    #[deku(count = "header.ancount")]
    pub answers: Vec<ResourceRecord>,
    #[deku(count = "header.nscount")]
    pub authorities: Vec<ResourceRecord>,
    #[deku(count = "header.arcount")]
    pub additionals: Vec<ResourceRecord>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct Header {
    pub id: u16,
    pub flags: Flags,
    pub qdcount: u16,
    pub ancount: u16,
    pub nscount: u16,
    pub arcount: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct Flags {
    #[deku(bits = "1")]
    pub qr: bool,
    pub opcode: Opcode,
    #[deku(bits = "1")]
    pub aa: bool,
    #[deku(bits = "1")]
    pub tc: bool,
    #[deku(bits = "1")]
    pub rd: bool,
    #[deku(bits = "1")]
    pub ra: bool,
    #[deku(bits = "3")]
    pub z: u8,
    pub rcode: RCode,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = "4")]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub enum Opcode {
    Query = 0,
    IQuery = 1,
    Status = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = "4")]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub enum RCode {
    NoError = 0,
    FormatError = 1,
    ServerFailure = 2,
    NameError = 3,
    NotImplemented = 4,
    Refused = 5,
}

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct Question {
    pub qname: Name,
    pub qtype: QType,
    pub qclass: QClass,
}

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct ResourceRecord {
    pub name: Name,
    pub r#type: QType,
    #[deku(cond = "*r#type != QType::OPT", default = "QClass::NONE")]
    pub qclass: QClass,
    pub ttl: i32,

    #[deku(update = "self.data.len()")]
    pub rdlength: u16,
    #[deku(count = "rdlength")]
    pub data: Vec<u8>,

    #[deku(cond = "*r#type == QType::OPT")]
    pub options_code: Option<u8>,
    #[deku(cond = "*r#type == QType::OPT")]
    pub options_length: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct Name {
    #[deku(until = "|label: &Label| label.len == 0")]
    labels: Vec<Label>,
}

impl Name {
    pub fn labels(&self) -> impl Iterator<Item = &str> {
        self.labels
            .iter()
            .filter(|label| label.len != 0)
            .map(|label| label.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct Label {
    #[deku(update = "self.data.len()")]
    pub len: u8,
    #[deku(count = "len")]
    pub data: Vec<u8>,
}

impl Label {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.data).unwrap()
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u16")]
#[deku(endian = "_endian", ctx = "_endian: deku::ctx::Endian")]
pub enum QType {
    A = 1,
    NS = 2,
    MD = 3,
    MF = 4,
    CNAME = 5,
    SOA = 6,
    MB = 7,
    MG = 8,
    MR = 9,
    NULL = 10,
    WKS = 11,
    PTR = 12,
    HINFO = 13,
    MINFO = 14,
    MX = 15,
    TXT = 16,
    OPT = 41,

    AXFR = 252,
    MAILB = 253,
    MAILA = 254,
    ANY = 255,
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u16")]
#[deku(endian = "_endian", ctx = "_endian: deku::ctx::Endian")]
pub enum QClass {
    NONE = 0,
    IN = 1,
    CS = 2,
    CH = 3,
    HS = 4,

    ANY = 255,
}

#[test]
fn decode_query() {
    use deku::bitvec::BitSlice;

    let data: &[u8] = &[
        100, 68, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 3, 102, 111, 111, 5, 108, 111, 99, 97, 108, 3, 100,
        101, 118, 0, 0, 255, 0, 1, 0, 0, 41, 2, 0, 0, 0, 0, 0, 0, 0,
    ];

    let ((rest, count), message) = Message::from_bytes((data, 0)).unwrap();
    println!("Message: {message:#?}");
    println!("Rest: {rest:?}");

    // let (rest, header) = Header::read(BitSlice::from_slice(data), deku::ctx::Endian::Big).unwrap();
    // println!("Header: {header:#?}");

    // let (rest, question) = Question::read(rest, deku::ctx::Endian::Big).unwrap();
    // println!("Question: {question:#?}");

    // let (rest, name) = Name::read(rest, deku::ctx::Endian::Big).unwrap();
    // println!("Name: {name:#?}");

    // let (rest, qtype) = QType::read(rest, deku::ctx::Endian::Big).unwrap();
    // dbg!(qtype);

    // let (rest, qclass) = QClass::read(rest, deku::ctx::Endian::Big).unwrap();
    // dbg!(qclass);
}
