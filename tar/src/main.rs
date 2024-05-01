use clap::Parser;
use std::fs::DirBuilder;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs::Permissions;
use std::io;
use std::io::prelude::*;
use std::io::stdin;
use std::io::stdout;
use std::io::BufReader;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;

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
    // let args = Cli::extract(String::from("test/files.tar"), String::from("/tmp"));

    if args.inspect || args.extract {
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

    let mut block = [0; 512];

    loop {
        let n = reader.read(&mut block)?;
        if n == 0 {
            break;
        }

        let file_name = block[0..99]
            .iter()
            .take_while(|&c| (*c as u8) != 0)
            .map(|&c| c)
            .collect::<Vec<u8>>();
        let file_mode = read_octal(&block[100..107]);
        let _owner_id = read_octal(&block[108..115]);
        let _group_id = read_octal(&block[116..123]);
        let file_size = read_octal(&block[124..135]);
        let last_modified_timestamp = read_octal(&block[136..147]);
        let _checksum = &block[148..155];
        let _link_idicator = &block[156];
        let _linked_file_name = &block[157..256];

        let file_name: String = match std::str::from_utf8(file_name.as_ref()) {
            Ok(v) => String::from(v),
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        if !file_name.is_empty() {
            print!("{}", file_name);

            let content_blocks = file_size.div_ceil(512);
            if args.extract {
                let root = args
                    .extract_dir
                    .clone()
                    .unwrap_or_else(|| String::from("."));
                let root = Path::new(root.as_str());
                let target_filepath = root.join(Path::new(&file_name));
                let mut ancestors = target_filepath.ancestors();
                ancestors.next(); // Drop filename
                if let Some(p) = ancestors.next() {
                    if !p.is_dir() {
                        DirBuilder::new().recursive(true).create(p).unwrap();
                    }
                }
                print!(" -> {:?}", target_filepath);
                let mut out = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(false)
                    .open(target_filepath)?;
                let mut remaining_bytes = file_size;
                for _ in 0..content_blocks {
                    reader.read(&mut block)?;
                    out.write(&block[0..remaining_bytes.min(511)])?;
                    remaining_bytes -= remaining_bytes.min(512);
                }
                out.set_permissions(Permissions::from_mode(file_mode as u32))
                    .unwrap();
                out.set_modified(
                    SystemTime::UNIX_EPOCH + Duration::from_secs(last_modified_timestamp as u64),
                )
                .unwrap();
                out.sync_all().unwrap();
            } else {
                for _ in 0..content_blocks {
                    reader.read(&mut block)?;
                }
            }
            println!();
        }
    }

    Ok(())
}

fn read_octal(bytes: &[u8]) -> usize {
    let str = std::str::from_utf8(bytes).unwrap();
    return usize::from_str_radix(str, 8).unwrap_or(0);
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
    #[arg(short = 'x', default_value_t = false, help = "Extract")]
    extract: bool,
    #[arg(short = 't', default_value_t = false, help = "Inspect")]
    inspect: bool,
    #[arg(short = 'c', default_value_t = false, help = "Create")]
    create: bool,
    #[arg(help = "Files to use")]
    files: Vec<String>,
    #[arg(short = 'C', help = "Extract directory")]
    extract_dir: Option<String>,
}

impl Cli {
    pub fn inspect(file: String) -> Self {
        Self {
            file: Some(file),
            inspect: true,
            extract: false,
            create: false,
            files: vec![],
            extract_dir: None,
        }
    }
    pub fn extract(file: String, extract_dir: String) -> Self {
        Self {
            file: Some(file),
            inspect: false,
            extract: true,
            create: false,
            files: vec![],
            extract_dir: Some(extract_dir),
        }
    }
    pub fn create(file: String, files: Vec<String>) -> Self {
        Self {
            file: Some(file),
            inspect: false,
            extract: false,
            create: true,
            files,
            extract_dir: None,
        }
    }
}
