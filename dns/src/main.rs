use std::{
    io::{Cursor, Read},
    net::UdpSocket,
};

const MAX_DATAGRAM_SIZE: usize = 65_507;
fn main() -> Result<(), std::io::Error> {
    let query = DnsMessage {
        header: Header {
            id: 42,
            flags: HeaderFlags {},
        },
        questions: vec![Question {
            domain: PlainDomainName(String::from("dns.google.com")),
            r#type: Qtype::HostAddress,
            class: Qclass::Internet,
        }],
        answers: vec![],
        authorities: vec![],
        additionals: vec![],
    }
    .to_bytes();
    for (_idx, byte) in query.iter().enumerate() {
        print!("{:02x}", byte);

        // if idx % 16 == 1 {
        //     println!("");
        // } else if idx % 2 == 1 {
        //     println!(" ");
        // }
    }
    // 002a0100000100000000000003646e7306676f6f676c6503636f6d0000010001
    // 00160100000100000000000003646e7306676f6f676c6503636f6d0000010001
    println!();

    println!("Here be dragons");
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:53")?;
    socket.send(&query)?;
    let mut response_buffer = vec![0u8; MAX_DATAGRAM_SIZE];
    let len = socket.recv(&mut response_buffer)?;
    println!("Received {} bytes:", len);
    for byte in response_buffer[..len].iter() {
        print!("{:02x}", byte);
    }
    println!();
    let mut cursor = Cursor::new(&response_buffer[0..len]);
    println!(
        "Parsed: {:?}",
        DnsMessage::parse(&response_buffer[0..len], &mut cursor)
    );
    Ok(())
}

trait Protocol {
    fn to_bytes(&self) -> Vec<u8>;
    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<Self, std::io::Error>
    where
        Self: Sized;
}

impl Protocol for u8 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![*self]
    }

    fn parse(_message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<u8, std::io::Error> {
        let mut buffer = [0];
        cursor.read_exact(&mut buffer)?;
        return Ok(buffer[0]);
    }
}

impl Protocol for u16 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![(*self >> 8) as u8, *self as u8]
    }

    fn parse(_message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<u16, std::io::Error> {
        let mut buffer = [0, 0];
        cursor.read_exact(&mut buffer)?;
        return Ok(((buffer[0] as u16) << 8) | (buffer[1] as u16));
    }
}

impl Protocol for u32 {
    fn to_bytes(&self) -> Vec<u8> {
        vec![
            (*self >> 24) as u8,
            (*self >> 16) as u8,
            (*self >> 8) as u8,
            *self as u8,
        ]
    }

    fn parse(_message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<u32, std::io::Error> {
        let mut buffer = [0, 0, 0, 0];
        cursor.read_exact(&mut buffer)?;
        return Ok(((buffer[0] as u32) << 24)
            | ((buffer[1] as u32) << 16)
            | ((buffer[2] as u32) << 8)
            | (buffer[3] as u32));
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DnsMessage {
    header: Header,
    questions: Vec<Question>,
    answers: Vec<Record>,
    authorities: Vec<Record>,
    additionals: Vec<Record>,
}
impl Protocol for DnsMessage {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(6 * 2);
        out.extend(self.header.to_bytes());

        out.extend((self.questions.len() as u16).to_bytes());
        out.extend((self.answers.len() as u16).to_bytes());
        out.extend((self.authorities.len() as u16).to_bytes());
        out.extend((self.additionals.len() as u16).to_bytes());

        for record in self.questions.iter() {
            out.extend(record.to_bytes());
        }

        for record in self.answers.iter() {
            out.extend(record.to_bytes());
        }

        for record in self.authorities.iter() {
            out.extend(record.to_bytes());
        }

        for record in self.additionals.iter() {
            out.extend(record.to_bytes());
        }

        return out;
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<DnsMessage, std::io::Error> {
        let header = Header::parse(message, cursor)?;

        let questions_count = (u16::parse(message, cursor)?) as usize;
        let answers_count = (u16::parse(message, cursor)?) as usize;
        let authorities_count = (u16::parse(message, cursor)?) as usize;
        let additionals_count = (u16::parse(message, cursor)?) as usize;

        let mut questions = Vec::with_capacity(questions_count);
        for _ in 0..questions_count {
            questions.push(Question::parse(message, cursor)?);
        }

        let mut answers = Vec::with_capacity(answers_count);
        for _ in 0..answers_count {
            answers.push(Record::parse(message, cursor)?);
        }

        let mut authorities = Vec::with_capacity(authorities_count);
        for _ in 0..authorities_count {
            authorities.push(Record::parse(message, cursor)?);
        }

        let mut additionals = Vec::with_capacity(additionals_count);
        for _ in 0..additionals_count {
            additionals.push(Record::parse(message, cursor)?);
        }

        Ok(DnsMessage {
            header,
            questions,
            answers,
            authorities,
            additionals,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Header {
    id: u16,
    flags: HeaderFlags,
}
impl Protocol for Header {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(6 * 2);
        out.extend(self.id.to_bytes());
        out.extend(self.flags.to_bytes());
        return out;
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<Header, std::io::Error> {
        let id = u16::parse(message, cursor)?;
        let flags = HeaderFlags::parse(message, cursor)?;
        Ok(Header { id, flags })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct HeaderFlags {}
impl Protocol for HeaderFlags {
    fn to_bytes(&self) -> Vec<u8> {
        return vec![1, 0]; // query / standard query / not authoritation / not truncated / no recursion / recusion available / zeros / no err code
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<HeaderFlags, std::io::Error> {
        let flags = u16::parse(message, cursor)?;
        assert_eq!(
            (flags & !(1 << 7)),  // is recursive available
            (1 << 15) | (1 << 8)  // response | authoritative
        );
        Ok(HeaderFlags {})
    }
}
/*
0016 => ID (22)
0100 => Flags (query / standard query / not authoritation / not truncated / no recursion / recusion available / zeros / no err code)
0001 => QDCOUNT (questions count)
0000 => ANCOUNT (answers count)
0000 => NSCOUNT (authorities count)
0000 => ARCOUNT (additional records)
*/

#[derive(Debug, Clone, PartialEq)]
struct Question {
    domain: PlainDomainName,
    r#type: Qtype,
    class: Qclass,
}
impl Protocol for Question {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend(self.domain.to_bytes());
        out.extend(self.r#type.to_bytes());
        out.extend(self.class.to_bytes());
        return out;
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<Question, std::io::Error> {
        let domain = PlainDomainName::parse(message, cursor)?;
        let type_ = Qtype::parse(message, cursor)?;
        let class = Qclass::parse(message, cursor)?;
        Ok(Question {
            domain,
            r#type: type_,
            class,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PlainDomainName(String);
impl Protocol for PlainDomainName {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        for part in self.0.split('.') {
            let bytes: Vec<u8> = part.bytes().collect();
            out.push(bytes.len() as u8);
            out.extend(bytes);
        }
        out.push(0); // end of domain
        return out;
    }

    fn parse(
        _message: &[u8],
        cursor: &mut Cursor<&[u8]>,
    ) -> Result<PlainDomainName, std::io::Error> {
        let mut domain_parts: Vec<Vec<u8>> = Vec::with_capacity(5); // usually 2-3, but let's be
                                                                    // generous
        loop {
            let mut len = [0];
            cursor.read_exact(&mut len)?;
            if len[0] == 0 {
                break;
            }
            let mut part = std::vec::from_elem(0, len[0] as usize);
            cursor.read_exact(&mut part)?;
            domain_parts.push(part);
        }
        Ok(PlainDomainName(
            domain_parts
                .iter()
                .map(|p| String::from_utf8_lossy(p.as_ref()).into_owned())
                .collect::<Vec<String>>()
                .join("."),
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Qtype {
    HostAddress,
}
impl Protocol for Qtype {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Qtype::HostAddress => vec![0, 1],
        }
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<Qtype, std::io::Error> {
        match u16::parse(message, cursor)? {
            1 => Ok(Qtype::HostAddress),
            n => todo!("Unsupported Qtype '{:04x}", n),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Qclass {
    Internet,
}
impl Protocol for Qclass {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Qclass::Internet => vec![0, 1],
        }
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<Qclass, std::io::Error> {
        match u16::parse(message, cursor)? {
            1 => Ok(Qclass::Internet),
            n => todo!("Unsupported Qclass '{:04x}", n),
        }
    }
}
/*
03 646e73 => dns
06 676f6f676c65 => google
03 636f6d => com
00

0001 => query type (host address)
0001 => query class (internet)
*/

#[derive(Debug, Clone, PartialEq)]
struct Record {
    domain: PlainDomainName,
    r#type: Qtype, // should be regular type
    class: Qclass, // should be regular class
    ttl: u32,
    data: String,
}
impl Protocol for Record {
    fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    fn parse(message: &[u8], cursor: &mut Cursor<&[u8]>) -> Result<Record, std::io::Error> {
        const COMPRESSION_ENABLED: u16 = (1 << 15) | (1 << 14);
        let pointer = u16::parse(message, cursor)?;
        let domain = match pointer & COMPRESSION_ENABLED {
            COMPRESSION_ENABLED => PlainDomainName::parse(
                message,
                &mut Cursor::new(&message[(pointer & !COMPRESSION_ENABLED) as usize..]),
            )?,
            0 => PlainDomainName::parse(message, cursor)?,
            _ => unreachable!("Invalid compression bits '{}'", pointer),
        };
        let type_ = Qtype::parse(message, cursor)?;
        let class = Qclass::parse(message, cursor)?;
        let ttl = u32::parse(message, cursor)?;

        let len = u16::parse(message, cursor)?;
        let mut data = std::vec::from_elem(0, len as usize);
        cursor.read_exact(&mut data)?;
        Ok(Record {
            domain,
            r#type: type_,
            class,
            ttl,
            data: String::from_utf8_lossy(data.as_ref()).into_owned(),
        })
    }
}
