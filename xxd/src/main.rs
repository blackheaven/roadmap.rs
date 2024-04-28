use clap::Parser;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;

fn main() -> io::Result<()> {
    let args = Cli::parse();
    // let mut args = Cli::new("test/files.tar".to_string());
    // args.bytes_per_line = 32;
    // args.max_read_bytes = 400;
    // args.bytes_per_group = 4;
    // args.little_endian = true;

    let f = File::open(args.file)?;
    let mut reader = BufReader::with_capacity(args.bytes_per_line, f);
    let mut offset = 0;

    loop {
        let buffer = reader.fill_buf()?;
        let n = buffer.len();
        if n == 0 || offset > args.max_read_bytes {
            reader.consume(n);
            break;
        }

        print!("{:010x}: ", offset);
        for g in 0..(args.bytes_per_line / args.bytes_per_group) {
            let base = g * args.bytes_per_group;
            print!(" ");
            for s in 0..args.bytes_per_group {
                let i = if args.little_endian {
                    base + ((s + args.bytes_per_group / 2) % args.bytes_per_group)
                } else {
                    s + base
                };
                if i < n {
                    print!("{:04x}", buffer[i]);
                } else {
                    print!("    ");
                }
            }
        }

        print!("  ");
        for i in 0..args.bytes_per_line {
            let c = buffer[i] as char;
            if i < n && !c.is_ascii_control() {
                print!("{}", c);
            } else {
                print!(".");
            }
        }
        println!();
        offset += args.bytes_per_line;
        reader.consume(n);
    }

    Ok(())
}

#[derive(Parser)] // requires `derive` feature
#[command(name = "xxd")]
#[command(bin_name = "xxd")]
// enum Cli {
//     Add(AddArgs),
//     Summary,
// }
//
// #[derive(clap::Args)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'f', help = "File to dump")]
    file: String,
    #[arg(short = 'e', default_value_t = false, help = "Little endian")]
    little_endian: bool,
    #[arg(short = 'g', default_value_t = 1, help = "Bytes per group")]
    bytes_per_group: usize,
    #[arg(
        short = 'c',
        default_value_t = 16,
        help = "Displayed byte counts per line"
    )]
    bytes_per_line: usize,
    #[arg(short = 'l', default_value_t = usize::MAX, help = "Max read bytes")]
    max_read_bytes: usize,
}

impl Cli {
    pub fn new(file: String) -> Self {
        Self {
            file,
            little_endian: false,
            bytes_per_group: 1,
            bytes_per_line: 16,
            max_read_bytes: usize::MAX,
        }
    }
}
