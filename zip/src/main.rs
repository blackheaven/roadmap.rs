use clap::Parser;
use std::borrow::Borrow;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::stdin;
use std::io::stdout;
use std::io::BufReader;

fn main() -> io::Result<()> {
    // let args = Cli::parse();
    let args = Cli::create(
        String::from("test/files.tar"),
        String::from("test/files.tar.hz"),
    );
    create(args)?;
    // let args = Cli::extract(
    //     String::from("test/files.tar.hz"),
    //     String::from("test/files_extracted.tar"),
    // );
    // extract(args)?;

    // if args.extract {
    //     extract(args)?
    // } else if args.create {
    //     create(args)?
    // }

    Ok(())
}

fn extract(args: Cli) -> io::Result<()> {
    let mut reader: Box<dyn BufRead> = match args.input {
        None => Box::new(BufReader::new(stdin())),
        Some(p) => Box::new(BufReader::new(File::open(p)?)),
    };

    let mut table = [0; 256 * EncodedTreePath::BYTES_SIZE];
    reader.read(&mut table)?;

    let tree = HuffmanTree::deserialize(
        (0..256)
            .into_iter()
            .map(|i| {
                (
                    i as u8,
                    EncodedTreePath::from_bytes(
                        table[i * EncodedTreePath::BYTES_SIZE
                            ..(i + 1) * EncodedTreePath::BYTES_SIZE]
                            .try_into()
                            .unwrap(),
                    ),
                )
            })
            .collect::<Vec<(u8, EncodedTreePath)>>(),
    );

    let mut raw_content_size = [0; 2];
    reader.read(&mut raw_content_size)?;
    let mut content_size = (raw_content_size[0] as usize) << 8 | (raw_content_size[1] as usize);
    let mut output: Box<dyn Write> = match args.output {
        None => Box::new(stdout()),
        Some(p) => Box::new(File::create(p)?),
    };

    if content_size == 0 {
        output.flush()?;
        return Ok(());
    }

    let mut search: &HuffmanTree = &tree;
    for bit in BitsIterator::new(reader) {
        match search.find(Choice::from_bit(bit)) {
            Ok(byte) => {
                output.write(&[byte])?;
                content_size -= 1;
                if content_size == 0 {
                    break;
                }
            }
            Err(next) => {
                search = next;
            }
        }
    }

    output.flush()?;
    Ok(())
}

fn create(args: Cli) -> io::Result<()> {
    let mut reader: Box<dyn BufRead> = match args.input {
        None => Box::new(BufReader::new(stdin())),
        Some(p) => Box::new(BufReader::new(File::open(p)?)),
    };

    let mut table = [0; 256];
    let mut file_content = Vec::with_capacity(4 * 1024 * 1024);
    let mut buffer = [0; 4 * 1024];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file_content.extend_from_slice(&buffer[0..n]);
    }

    for byte in file_content.iter() {
        table[*byte as usize] += 1;
    }

    let tree = HuffmanTree::new(
        (0..256)
            .into_iter()
            .map(|i| (i as u8, table[i]))
            .collect::<Vec<(u8, u64)>>(),
    );

    let mut output: Box<dyn Write> = match args.output.clone() {
        None => Box::new(stdout()),
        Some(p) => Box::new(File::create(p)?),
    };

    let vtable = tree
        .serialize()
        .iter()
        .flat_map(EncodedTreePath::to_bytes)
        .collect::<Vec<u8>>();
    output.write(vtable.as_ref())?;
    output.write(&[(file_content.len() >> 8) as u8, file_content.len() as u8])?;
    let mut content_size = file_content.len();

    let lookup_tree = tree.serialize_choices();
    let mut compressed = Vec::with_capacity(file_content.len());
    compressed.push(0);
    let mut dbg = 13;
    let mut bit = 0;
    for byte in file_content.iter() {
        for choice in lookup_tree[*byte as usize].iter() {
            if bit == 8 {
                compressed.push(0);
                bit = 0;
            }
            if dbg > 0 {
                print!("{:?}", choice.to_bit());
            }
            let last = compressed.len() - 1;
            compressed[last] = (compressed[last] << 1) | choice.to_bit();
            bit += 1;
        }
        if dbg > 0 {
            println!(" {}", *byte);
            dbg -= 1;
        }
    }

    output.write(compressed.as_ref())?;

    output.flush()?;

    let mut reader = BufReader::new(File::open(args.output.clone().unwrap())?);
    let mut unzipped = File::create("test/files_extracted.tar")?;
    reader.seek_relative((256 * EncodedTreePath::BYTES_SIZE + 2) as i64)?;

    let mut dbg = 13;
    let mut search: &HuffmanTree = &tree;
    for bit in BitsIterator::new(reader) {
        match search.find(Choice::from_bit(bit)) {
            Ok(byte) => {
                if dbg > 0 {
                    println!("{}", byte);
                    dbg -= 1;
                }
                unzipped.write(&[byte])?;
                content_size -= 1;
                search = &tree;
                if content_size == 0 {
                    break;
                }
            }
            Err(next) => {
                search = next;
            }
        }
    }

    unzipped.flush()?;

    Ok(())
}

#[derive(Parser)] // requires `derive` feature
#[command(name = "hzip")]
#[command(bin_name = "hzip")]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'i', help = "Input file to use")]
    input: Option<String>,
    #[arg(short = 'o', help = "Output file to use")]
    output: Option<String>,
    #[arg(short = 'x', default_value_t = false, help = "Extract")]
    extract: bool,
    #[arg(short = 'c', default_value_t = false, help = "Create")]
    create: bool,
}

impl Cli {
    pub fn create(input: String, output: String) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            extract: false,
            create: true,
        }
    }
    pub fn extract(input: String, output: String) -> Self {
        Self {
            input: Some(input),
            output: Some(output),
            extract: true,
            create: false,
        }
    }
}

enum HuffmanTree {
    Node {
        weight: u64,
        left: Box<HuffmanTree>,
        right: Box<HuffmanTree>,
    },
    Leaf {
        weight: u64,
        byte: u8,
    },
}

#[derive(Clone, Debug, Copy)]
enum Choice {
    Left,
    Right,
}

impl Choice {
    pub fn from_bit(byte: u8) -> Self {
        if byte & 1 == 1 {
            Choice::Right
        } else {
            Choice::Left
        }
    }

    pub fn to_bit(&self) -> u8 {
        match self {
            Choice::Left => 0,
            Choice::Right => 1,
        }
    }

    pub fn serialize(choices: Vec<Choice>) -> EncodedTreePath {
        let mut bytes = EncodedTreePath::new();
        for choice in choices.iter().rev() {
            bytes = bytes.shl(1) | EncodedTreePath::from_byte(choice.to_bit());
        }
        return bytes;
    }

    pub fn deserialize(mut bytes: EncodedTreePath) -> Vec<Choice> {
        // Leftmost bits became last vec item
        if bytes.is_zero() {
            return vec![Choice::Left];
        }
        let mut choices = Vec::with_capacity(256);
        while !bytes.is_zero() {
            choices.push(Self::from_bit(bytes.lowest_byte()));
            bytes = bytes.shr(1);
        }
        assert!(choices.len() > 0);
        return choices;
    }
}

impl HuffmanTree {
    pub fn new(mut frequencies: Vec<(u8, u64)>) -> Self {
        frequencies.sort_by_key(|(_, frequency)| *frequency);
        let mut it = frequencies.iter();
        let mut root = {
            let (byte, weight) = it.next().unwrap();
            Self::Leaf {
                weight: *weight,
                byte: *byte,
            }
        };
        for (byte, weight) in it {
            root = if *weight >= root.weight() {
                Self::Node {
                    weight: weight + root.weight(),
                    left: Box::new(root),
                    right: Box::new(Self::Leaf {
                        weight: *weight,
                        byte: *byte,
                    }),
                }
            } else {
                Self::Node {
                    weight: weight + root.weight(),
                    left: Box::new(Self::Leaf {
                        weight: *weight,
                        byte: *byte,
                    }),
                    right: Box::new(root),
                }
            };
        }
        return root;
    }

    pub fn weight(&self) -> u64 {
        match self {
            Self::Node { weight, .. } => *weight,
            Self::Leaf { weight, .. } => *weight,
        }
    }

    pub fn find(&self, choice: Choice) -> Result<u8, &HuffmanTree> {
        match self {
            Self::Node { left, right, .. } => {
                let sub = match choice {
                    Choice::Left => left.borrow(),
                    Choice::Right => right.borrow(),
                };
                match sub {
                    Self::Node { .. } => Err(sub),
                    Self::Leaf { byte, .. } => Ok(*byte),
                }
            }
            Self::Leaf { .. } => unreachable!(),
        }
    }

    pub fn deserialize(table: Vec<(u8, EncodedTreePath)>) -> Self {
        Self::deserialize_with(
            table
                .clone()
                .iter()
                .map(|(byte, choices)| (*byte, Choice::deserialize(*choices)))
                .collect(),
        )
    }

    fn deserialize_with(table: Vec<(u8, Vec<Choice>)>) -> Self {
        match table.len() {
            0 => unreachable!(),
            1 => Self::Leaf {
                weight: 0,
                byte: table[0].0,
            },
            _ => {
                let mut lefts = Vec::with_capacity(table.len());
                let mut rights = Vec::with_capacity(table.len());
                let mut leftover = None;
                for (byte, choices) in table.iter() {
                    let mut choices = choices.clone();
                    if choices.len() == 0 {
                        leftover = Some(byte);
                        continue;
                    }
                    match choices.remove(0) {
                        Choice::Left => lefts.push((*byte, choices)),
                        Choice::Right => rights.push((*byte, choices)),
                    }
                }
                match leftover {
                    None => Self::Node {
                        weight: 0,
                        left: Box::new(Self::deserialize_with(lefts)),
                        right: Box::new(Self::deserialize_with(rights)),
                    },
                    Some(byte) => {
                        if rights.is_empty() {
                            Self::Node {
                                weight: 0,
                                left: Box::new(Self::deserialize_with(lefts)),
                                right: Box::new(Self::Leaf {
                                    weight: 0,
                                    byte: *byte,
                                }),
                            }
                        } else {
                            Self::Node {
                                weight: 0,
                                left: Box::new(Self::Leaf {
                                    weight: 0,
                                    byte: *byte,
                                }),
                                right: Box::new(Self::deserialize_with(rights)),
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn serialize(&self) -> Vec<EncodedTreePath> {
        return self
            .serialize_choices()
            .iter()
            .map(|choices: &Vec<Choice>| Choice::serialize(choices.clone()))
            .collect();
    }

    pub fn serialize_choices(&self) -> Vec<Vec<Choice>> {
        let mut map_choices = self.serialize_choices_with(vec![]);
        map_choices.sort_by_key(|(byte, _)| *byte);
        return map_choices
            .iter()
            .map(|(_, choices)| choices.clone())
            .collect();
    }

    fn serialize_choices_with(&self, choices: Vec<Choice>) -> Vec<(u8, Vec<Choice>)> {
        match self {
            Self::Node { left, right, .. } => {
                let mut left_choices = choices.clone();
                left_choices.push(Choice::Left);
                let mut right_choices = choices.clone();
                right_choices.push(Choice::Right);
                let mut serialized = left.serialize_choices_with(left_choices);
                serialized.extend(right.serialize_choices_with(right_choices));
                return serialized;
            }
            Self::Leaf { byte, .. } => vec![(*byte, choices)],
        }
    }
}

struct BitsIterator<T> {
    reader: T,
    buffer: [u8; 1],
    bit: usize,
}

impl<T> BitsIterator<T> {
    fn new(reader: T) -> BitsIterator<T> {
        BitsIterator {
            reader,
            buffer: [0],
            bit: 0,
        }
    }
}

impl<T: BufRead> Iterator for BitsIterator<T> {
    // we will be counting with usize
    type Item = u8;

    // next() is the only required method
    fn next(&mut self) -> Option<Self::Item> {
        if self.bit == 8 {
            let n = self.reader.read(&mut self.buffer).unwrap();
            if n == 0 {
                return None;
            }
            self.bit = 0;
        }

        let bit = self.buffer[0] & 1;
        self.buffer[0] = self.buffer[0] >> 1;
        self.bit += 1;

        return Some(bit);
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct EncodedTreePath(u128, u128);

impl EncodedTreePath {
    pub const BYTES_SIZE: usize = 32;

    pub fn new() -> Self {
        Self(0, 0)
    }
    pub fn from_byte(byte: u8) -> Self {
        Self(0, byte as u128)
    }
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(
            u128::from_be_bytes(bytes[0..16].try_into().unwrap()),
            u128::from_be_bytes(bytes[16..32].try_into().unwrap()),
        )
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out[0..16].clone_from_slice(&self.0.to_be_bytes());
        out[16..32].clone_from_slice(&self.1.to_be_bytes());
        return out;
    }

    pub fn is_zero(&self) -> bool {
        self.0 == 0 && self.1 == 0
    }

    pub fn lowest_byte(&self) -> u8 {
        self.1 as u8
    }

    fn shl(self, shift: u8) -> Self {
        let mut high = self.0;
        let mut low = self.1;
        let highest = 1 << 127;
        for _ in 0..shift {
            let overflow = if low & highest == 0 { 0 } else { 1 };
            high = (high << 1) | overflow;
            low = low << 1;
        }
        return Self(high, low);
    }

    fn shr(self, shift: u8) -> Self {
        let mut high = self.0;
        let mut low = self.1;
        let lowest = 1;
        for _ in 0..shift {
            let underflow = if high & lowest == 0 { 0 } else { 1 << 127 };
            high = high >> 1;
            low = (low >> 1) | underflow;
        }
        return Self(high, low);
    }
}

impl std::ops::BitOr for EncodedTreePath {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0, self.1 | rhs.1)
    }
}

impl std::ops::BitAnd for EncodedTreePath {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0, self.1 & rhs.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_serialization_roundtrip_test() {
        for raw_bytes in 0..u16::MAX {
            let original_bytes = EncodedTreePath::from_bytes([
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                (raw_bytes >> 8) as u8,
                raw_bytes as u8,
            ]);
            let original_choices = Choice::deserialize(original_bytes);
            let reserialized_bytes = Choice::serialize(original_choices.clone());
            if original_bytes != reserialized_bytes {
                println!(
                    "Original {:?} => {:?}",
                    original_bytes.to_bytes(),
                    original_choices.clone()
                );
                println!(
                    "Reserialized {:?} => {:?}",
                    reserialized_bytes.to_bytes(),
                    Choice::deserialize(reserialized_bytes)
                );
                assert_eq!(original_bytes, reserialized_bytes);
            }
        }
    }

    #[test]
    fn encoded_table_seralization_test() {
        for byte in 0..u8::MAX {
            let raw = [byte; 32];
            assert_eq!(raw, EncodedTreePath::from_bytes(raw).to_bytes());
        }
    }

    #[test]
    fn encoded_table_shl_no_overflow_test() {
        let original = EncodedTreePath::from_bytes([
            0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0,
            7, 0, 7,
        ]);
        let expected = EncodedTreePath::from_bytes([
            7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7,
            0, 7, 0,
        ]);
        assert_eq!(original.shl(8), expected);
    }

    #[test]
    fn encoded_table_shl_overflow_test() {
        let original = EncodedTreePath::from_bytes([
            0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0,
            7, 0, 7,
        ]);
        let expected = EncodedTreePath::from_bytes([
            0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0,
            7, 0, 0,
        ]);
        assert_eq!(original.shl(16), expected);
    }

    #[test]
    fn encoded_table_shl_no_underflow_test() {
        let original = EncodedTreePath::from_bytes([
            7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7,
            0, 7, 0,
        ]);
        let expected = EncodedTreePath::from_bytes([
            0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0,
            7, 0, 7,
        ]);
        assert_eq!(original.shr(8), expected);
    }

    #[test]
    fn encoded_table_shl_underflow_test() {
        let original = EncodedTreePath::from_bytes([
            7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7,
            0, 7, 0,
        ]);
        let expected = EncodedTreePath::from_bytes([
            0, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7, 0, 7,
            0, 7, 0,
        ]);
        assert_eq!(original.shr(16), expected);
    }
}
