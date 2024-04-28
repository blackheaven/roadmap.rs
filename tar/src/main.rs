use clap::Parser;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::stdin;
use std::io::stdout;
use std::io::BufReader;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

fn main() -> io::Result<()> {
    let args = Cli::parse();
    // let args = Cli::inspect(String::from("test/files.tar"));
    // let args = Cli::inspect(String::from("test/files_generated.tar"));
    // let args = Cli::create(
    //     String::from("test/files_generated.tar"),
    //     vec![
    //         String::from("test/file1.txt"),
    //         String::from("test/file2.txt"),
    //         String::from("test/file3.txt"),
    //     ],
    // );

    if args.inspect {
        inspect(args)?
    } else if args.create {
        create(args)?
    }

    Ok(())
}

const BLOCKS_PER_RECORD: usize = 20;

fn create(args: Cli) -> io::Result<()> {
    let mut writer: Box<dyn Write> = match args.file {
        None => Box::new(stdout()),
        Some(p) => Box::new(File::create(p)?),
    };

    let mut blocks = 0;
    let mut block = [0; 512];
    for file_name in args.files {
        block = [0; 512];

        let f = File::open(file_name.clone())?;
        let metadata = f.metadata()?;
        let mut header_file_name = file_name.clone();
        header_file_name.truncate(100);
        write!(&mut block[0..99], "{}", header_file_name)?;
        write!(
            &mut block[100..107],
            "{:07o}",
            (metadata.permissions().mode() & 0x7FF) // truncate sticky bits or so
        )?;
        write!(&mut block[108..115], "{:07o}", metadata.uid())?;
        write!(&mut block[116..123], "{:07o}", metadata.gid())?;
        write!(&mut block[124..135], "{:011o}", metadata.size())?;
        write!(&mut block[136..147], "{:011o}", metadata.mtime())?;
        // let _link_idicator = &block[156];
        // let _linked_file_name = &block[157..256];

        // ustar part
        block[155] = b' '; // normal file
        block[156] = b'0'; // normal file
        write!(&mut block[257..262], "{}", "ustar")?;
        write!(&mut block[262..264], "{}", "  ")?; // version
                                                   // block[263] = b' '; // version
                                                   // block[264] = b' '; // version
        write!(&mut block[265..296], "{}", "black")?;
        write!(&mut block[297..328], "{}", "users")?;

        // commented lines above, should be write before checksum computation

        // The checksum is calculated by taking the sum of the unsigned byte values of the header record with the eight checksum bytes taken to be ASCII spaces
        let checksum: u64 = 7 * (b' ' as u64) + block.iter().map(|n| *n as u64).sum::<u64>();
        write!(&mut block[148..155], "{:06o}", checksum)?; // should truncate

        writer.write(&block)?;
        blocks += 1;

        let mut reader = BufReader::new(f);
        loop {
            block = [0; 512];
            let n = reader.read(&mut block)?;
            if n == 0 {
                break;
            }

            writer.write(&block)?;
            blocks += 1;
        }
    }

    block = [0; 512];
    for _ in 0..(BLOCKS_PER_RECORD - (blocks % BLOCKS_PER_RECORD)) {
        writer.write(&block)?;
    }

    Ok(())
}

fn inspect(args: Cli) -> io::Result<()> {
    let mut reader: Box<dyn BufRead> = match args.file {
        None => Box::new(BufReader::new(stdin())),
        Some(p) => Box::new(BufReader::new(File::open(p)?)),
    };
    // let f = File::open(args.file.clone().unwrap())?;
    // let mut reader = BufReader::new(f);
    let mut block = [0; 512];
    let mut content_block = 0;
    let mut nb = 0;

    loop {
        let n = reader.read(&mut block)?;
        if n == 0 {
            break;
        }
        nb += 1;

        if content_block == 0 {
            let file_name = &block[0..99];
            let _file_mode_octal = &block[100..107];
            let _owner_id_octal = &block[108..115];
            let _group_id_octal = &block[116..123];
            let file_size_octal = &block[124..135];
            let _last_modified_timestamp_octal = &block[136..147];
            let _checksum = &block[148..155];
            let _link_idicator = &block[156];
            let _linked_file_name = &block[157..256];

            let s: String = match std::str::from_utf8(file_name) {
                Ok(v) => v.chars().take_while(|&c| (c as u8) != 0).collect(),
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
            if !s.is_empty() {
                println!("{}", s);
            }

            let file_size_str = std::str::from_utf8(file_size_octal).unwrap();
            content_block = match usize::from_str_radix(file_size_str, 8) {
                Ok(n) => n.div_ceil(512),
                Err(_) => 0,
            }
        } else {
            content_block -= 1;
        }
    }

    println!("{}", nb);
    Ok(())
}

#[derive(Parser)] // requires `derive` feature
#[command(name = "tar")]
#[command(bin_name = "tar")]
// enum Cli {
//     Add(AddArgs),
//     Summary,
// }
//
// #[derive(clap::Args)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short = 'f', help = "Tar file to use")]
    file: Option<String>,
    #[arg(short = 't', default_value_t = false, help = "Inspect")]
    inspect: bool,
    #[arg(short = 'c', default_value_t = false, help = "Create")]
    create: bool,
    #[arg(help = "Files to use")]
    files: Vec<String>,
}

impl Cli {
    pub fn inspect(file: String) -> Self {
        Self {
            file: Some(file),
            inspect: true,
            create: false,
            files: vec![],
        }
    }
    pub fn create(file: String, files: Vec<String>) -> Self {
        Self {
            file: Some(file),
            inspect: false,
            create: true,
            files,
        }
    }
}
