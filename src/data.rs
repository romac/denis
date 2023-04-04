#![allow(clippy::upper_case_acronyms)]

use core::fmt;
use std::str::FromStr;

use color_eyre::{eyre::eyre, Report};
use deku::{
    bitvec::{BitSlice, BitVec, Msb0},
    prelude::*,
};

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
pub struct Message {
    pub header: Header,
    #[deku(count = "header.qdcount", read_ctx = "deku::input_bits")]
    pub questions: Vec<Question>,
    #[deku(count = "header.ancount", read_ctx = "deku::input_bits")]
    pub answers: Vec<ResourceRecord>,
    #[deku(count = "header.nscount", read_ctx = "deku::input_bits")]
    pub authorities: Vec<ResourceRecord>,
    #[deku(count = "header.arcount", read_ctx = "deku::input_bits")]
    pub additionals: Vec<ResourceRecord>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
pub struct Header {
    #[deku(endian = "big")]
    pub id: u16,
    pub flags: Flags,
    #[deku(endian = "big")]
    pub qdcount: u16,
    #[deku(endian = "big")]
    pub ancount: u16,
    #[deku(endian = "big")]
    pub nscount: u16,
    #[deku(endian = "big")]
    pub arcount: u16,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
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
    #[deku(bits = "3", endian = "big")]
    pub z: u8,
    pub rcode: RCode,
}

impl Flags {
    pub fn answer(opcode: Opcode) -> Self {
        Self {
            qr: true,
            opcode,
            aa: true,
            tc: false,
            rd: false,
            ra: false,
            z: 0,
            rcode: RCode::NoError,
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = "4")]
pub enum Opcode {
    Query = 0,
    IQuery = 1,
    Status = 2,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(type = "u8", bits = "4")]
pub enum RCode {
    NoError = 0,
    FormatError = 1,
    ServerFailure = 2,
    NameError = 3,
    NotImplemented = 4,
    Refused = 5,
}

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(read_ctx = "input: &'__deku_input BitSlice<u8, Msb0>")]
pub struct Question {
    #[deku(read_ctx = "input")]
    pub qname: Name,
    pub qtype: QType,
    pub qclass: QClass,
}

#[derive(Clone, Debug, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(read_ctx = "input: &'__deku_input BitSlice<u8, Msb0>")]
pub struct ResourceRecord {
    #[deku(read_ctx = "input")]
    pub name: Name,

    pub qtype: QType,

    #[deku(cond = "*qtype != QType::OPT", default = "QClass::NONE")]
    pub qclass: QClass,

    #[deku(endian = "big")]
    pub ttl: i32,

    #[deku(update = "self.data.len()", endian = "big")]
    pub rdlength: u16,
    #[deku(count = "rdlength")]
    pub data: Vec<u8>,

    #[deku(cond = "*qtype == QType::OPT")]
    pub options_code: Option<u8>,
    #[deku(cond = "*qtype == QType::OPT")]
    pub options_length: Option<u8>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Name {
    labels: Vec<Label>,
}

impl FromStr for Name {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

impl Name {
    pub fn new(data: String) -> Self {
        let labels = data
            .split('.')
            .map(|label| Label::new(label.to_string()))
            .collect();

        Self { labels }
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    pub fn labels(&self) -> &[Label] {
        self.labels.as_slice()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = deku::bitvec::BitVec::new();
        deku::DekuWrite::write(self, &mut output, ()).unwrap();
        output.into_vec()
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, ".");
        }

        let mut labels = self.labels.iter();

        if let Some(label) = labels.next() {
            write!(f, "{}", label)?;
        }

        for label in labels {
            write!(f, ".{}", label)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<'a, '__deku_input> DekuRead<'a, &'__deku_input BitSlice<u8, Msb0>> for Name {
    fn read(
        input: &'a BitSlice<u8, Msb0>,
        ctx: &'__deku_input BitSlice<u8, Msb0>,
    ) -> Result<(&'a BitSlice<u8, Msb0>, Self), DekuError>
    where
        Self: Sized,
    {
        let (input, len) = u8::read(input, ())?;

        if len == 0 {
            return Ok((input, Self { labels: vec![] }));
        }

        if len & 0b1100_0000 == 0b1100_0000 {
            let len = len & 0b0011_1111;

            let (input, offset) = u8::read(input, ())?;
            let offset = (len | offset) as usize * 8;

            let (_, labels) = parse_labels(&ctx[offset..], len)?;
            Ok((input, Self { labels }))
        } else {
            let (input, labels) = parse_labels(input, len)?;
            Ok((input, Self { labels }))
        }
    }
}

fn parse_labels(
    input: &BitSlice<u8, Msb0>,
    initial_len: u8,
) -> Result<(&BitSlice<u8, Msb0>, Vec<Label>), DekuError> {
    if initial_len == 0 {
        return Ok((input, vec![]));
    }

    let mut labels = Vec::new();
    let mut input = input;

    let data = input[0..initial_len as usize * 8].to_bitvec().into_vec();
    labels.push(Label::new(String::from_utf8(data).unwrap()));
    input = &input[initial_len as usize * 8..];

    loop {
        let (rest, len) = u8::read(input, ())?;

        if len == 0 {
            input = rest;
            break;
        }

        let data = rest[0..len as usize * 8].to_bitvec().into_vec();
        labels.push(Label::new(String::from_utf8(data).unwrap()));
        input = &rest[len as usize * 8..];
    }

    Ok((input, labels))
}

impl DekuWrite for Name {
    fn write(&self, output: &mut BitVec<u8, Msb0>, _ctx: ()) -> Result<(), DekuError>
    where
        Self: Sized,
    {
        for label in &self.labels {
            u8::write(&(label.as_str().len() as u8), output, ())?;
            output.extend_from_raw_slice(label.as_str().as_bytes());
        }

        u8::write(&0, output, ())?;

        Ok(())
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Label(String);

impl Label {
    pub fn new(data: String) -> Self {
        if data.len() > 63 {
            panic!("Label too long");
        }

        Self(data)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.as_str())
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, DekuRead, DekuWrite)]
#[deku(type = "u16", endian = "big")]
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
    AAAA = 28,
    OPT = 41,
    SVCB = 64,
    HTTPS = 65,

    AXFR = 252,
    MAILB = 253,
    MAILA = 254,
    ANY = 255,
}

impl fmt::Display for QType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl FromStr for QType {
    type Err = Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(QType::A),
            "NS" => Ok(QType::NS),
            "MD" => Ok(QType::MD),
            "MF" => Ok(QType::MF),
            "CNAME" => Ok(QType::CNAME),
            "SOA" => Ok(QType::SOA),
            "MB" => Ok(QType::MB),
            "MG" => Ok(QType::MG),
            "MR" => Ok(QType::MR),
            "NULL" => Ok(QType::NULL),
            "WKS" => Ok(QType::WKS),
            "PTR" => Ok(QType::PTR),
            "HINFO" => Ok(QType::HINFO),
            "MINFO" => Ok(QType::MINFO),
            "MX" => Ok(QType::MX),
            "TXT" => Ok(QType::TXT),
            "AAAA" => Ok(QType::AAAA),
            "OPT" => Ok(QType::OPT),
            "SVCB" => Ok(QType::SVCB),
            "HTTPS" => Ok(QType::HTTPS),
            "AXFR" => Ok(QType::AXFR),
            "MAILB" => Ok(QType::MAILB),
            "MAILA" => Ok(QType::MAILA),
            "ANY" => Ok(QType::ANY),
            s => Err(eyre!("Invalid QType: {s}")),
        }
    }
}

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, DekuRead, DekuWrite)]
#[deku(type = "u16", endian = "big")]
pub enum QClass {
    NONE = 0,
    IN = 1,
    CS = 2,
    CH = 3,
    HS = 4,

    ANY = 255,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_query() {
        let data: &[u8] = &[
            100, 68, 1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 3, 102, 111, 111, 5, 108, 111, 99, 97, 108, 3,
            100, 101, 118, 0, 0, 255, 0, 1, 0, 0, 41, 2, 0, 0, 0, 0, 0, 0, 0,
        ];

        let ((rest, _count), message) = Message::from_bytes((data, 0)).unwrap();
        println!("Message: {message:#?}");
        println!("Rest: {rest:?}");
    }

    #[test]
    fn decode_google() {
        let data: &[u8] = &[
            13, 208, 129, 128, 0, 1, 0, 1, 0, 0, 0, 0, 4, 110, 101, 119, 115, 11, 121, 99, 111,
            109, 98, 105, 110, 97, 116, 111, 114, 3, 99, 111, 109, 0, 0, 1, 0, 1, 192, 12, 0, 1, 0,
            1, 0, 0, 0, 1, 0, 4, 209, 216, 230, 240,
        ];

        let (_, message) = Message::from_bytes((data, 0)).unwrap();
        println!("Message: {message:#?}");
    }

    #[test]
    fn decode_unknown() {
        // let data: &[u8] = &[
        //     187, 76, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 3, 97, 112, 105, 14, 97, 112, 112, 108, 101, 45,
        //     99, 108, 111, 117, 100, 107, 105, 116, 2, 102, 101, 9, 97, 112, 112, 108, 101, 45, 100,
        //     110, 115, 3, 110, 101, 116, 0, 0, 1, 0, 1,
        // ];

        let data: &[u8] = &[
            58, 211, 129, 128, 0, 1, 0, 2, 0, 1, 0, 0, 13, 99, 111, 110, 102, 105, 103, 117, 114,
            97, 116, 105, 111, 110, 2, 108, 115, 5, 97, 112, 112, 108, 101, 3, 99, 111, 109, 0, 0,
            65, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 13, 244, 0, 37, 10, 103, 115, 112, 101, 49, 49,
            45, 115, 115, 108, 2, 108, 115, 5, 97, 112, 112, 108, 101, 3, 99, 111, 109, 7, 101,
            100, 103, 101, 107, 101, 121, 3, 110, 101, 116, 0, 192, 56, 0, 5, 0, 1, 0, 0, 84, 68,
            0, 26, 6, 101, 49, 48, 52, 57, 57, 5, 100, 115, 99, 101, 57, 10, 97, 107, 97, 109, 97,
            105, 101, 100, 103, 101, 192, 88, 192, 112, 0, 6, 0, 1, 0, 0, 3, 204, 0, 50, 7, 110,
            48, 100, 115, 99, 101, 57, 192, 118, 10, 104, 111, 115, 116, 109, 97, 115, 116, 101,
            114, 6, 97, 107, 97, 109, 97, 105, 192, 35, 100, 43, 245, 193, 0, 0, 3, 232, 0, 0, 3,
            232, 0, 0, 3, 232, 0, 0, 7, 8,
        ];

        let (_, message) = Message::from_bytes((data, 0)).unwrap();
        println!("Message: {message:#?}");
    }
}
