use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::convert::TryInto;
use std::collections::HashSet;

fn print_usage() -> Result<(), String> {
    eprint!("\
dream-sparer: RIFX file reader {} by hikari_no_yume. Copyright 2021.
MIT licensed.

Usage:
    dream-sparer path/to/rifx/file

With no other arguments passed, dream-sparer will just print a list of chunks
found in the file.

Optional arguments:

  --help        Display this help
  --quiet=XXXX  Don't print anything for chunk type XXXX.
                You can use this argument multiple times (for multiple types).
  --dump=XXXX   When encountering a chunk of type XXXX, dump it to a file.
                The files will be named like 0000_1234.XXXX where 0000 is the
                index and 1234 is the byte offset within the file.
                You can use this argument multiple times (for multiple types).
", env!("CARGO_PKG_VERSION"));
    Ok(())
}

fn convert_io_error(e: std::io::Error) -> String {
    format!("Error when reading/writing file: {}", e)
}

type FourCC = [u8; 4];

fn convert_fourcc(arg: &str) -> Result<FourCC, String> {
    if arg.len() != 4 || !arg.is_ascii() {
        Err(format!("'{}' is not 4 bytes long / is not ASCII.", arg))
    } else {
        Ok(arg.as_bytes().try_into().unwrap())
    }
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return print_usage();
    }
    let mut filename: Option<&str> = None;
    let mut quiet_fourccs: HashSet<FourCC> = HashSet::new();
    let mut dump_fourccs: HashSet<FourCC> = HashSet::new();
    for arg in &args[1..] {
        if arg == "--help" {
            return print_usage();
        } else if let Some(fourcc) = arg.strip_prefix("--quiet=") {
            quiet_fourccs.insert(convert_fourcc(fourcc)?);
        } else if let Some(fourcc) = arg.strip_prefix("--dump=") {
            dump_fourccs.insert(convert_fourcc(fourcc)?);
        } else if arg.starts_with("--") {
            return Err(format!("Unknown argument: '{}'", arg));
        } else {
            match filename {
                Some(_)  => {
                    return Err(format!("Only one filename can be specified."));
                },
                None => {
                    filename = Some(arg);
                }
            }
        }
    }

    if let Some(filename) = filename {
        let mut file = File::open(filename).map_err(convert_io_error)?;

        read_riff_file(&mut file, &quiet_fourccs, &dump_fourccs)
    } else {
        Err(format!("No filename was specified."))
    }
}

fn format_fourcc(f: FourCC) -> String {
    if f.is_ascii() {
        format!(
            "'{}'",
            unsafe { std::str::from_utf8_unchecked(&f) }
        )
    } else {
        format!("{:?}", f)
    }
}

const XFIR: FourCC = [b'X', b'F', b'I', b'R'];

fn read_fourcc(f: &mut File, byteswap: bool) -> Result<FourCC, String> {
    let mut buffer = [0u8; 4];
    f.read_exact(&mut buffer).map_err(convert_io_error)?;
    if byteswap {
        buffer.reverse();
    }
    Ok(buffer)
}

fn read_u32_le(f: &mut File) -> Result<u32, String> {
    let mut buffer = [0u8; 4];
    f.read_exact(&mut buffer).map_err(convert_io_error)?;
    // Yes this is unsafe. Yes this is the best way to do it.
    Ok(u32::from_le(unsafe { std::mem::transmute::<[u8; 4], u32>(buffer) }))
}

fn read_riff_file(
    f: &mut File,
    quiet_fourccs: &HashSet<FourCC>,
    dump_fourccs: &HashSet<FourCC>,
) -> Result<(), String> {
    let file_type = read_fourcc(f, false)?;
    print!("File's magic number/FourCC is {}: ", format_fourcc(file_type));
    if file_type == XFIR {
        println!("Little-endian RIFX file.");
    } else {
        println!("Unknown.");
        return Err(format!("This format is not supported yet."));
    }

    let file_size = read_u32_le(f)?;
    println!("File size according to RIFF header: {} bytes", file_size);

    let file_kind = read_fourcc(f, true)?;
    println!("File kind according to RIFF header: {}", format_fourcc(file_kind));

    let mut offset = 12;
    let mut index = 0;
    while offset < file_size {
        let chunk_type = read_fourcc(f, true)?;
        let chunk_size = read_u32_le(f)?;
        let chunk_offset = offset;
        offset += 8;

        let quiet = quiet_fourccs.contains(&chunk_type);
        if !quiet {
            println!(
                "Chunk #{} of type {}, size {} bytes at offset {} bytes",
                index,
                format_fourcc(chunk_type),
                chunk_size,
                chunk_offset
            );
        }

        // RIFF pads chunk sizes to be 2-byte-aligned (the era of “DWORDs”…)
        let seek_size = chunk_size + (chunk_size & 1);

        if !dump_fourccs.contains(&chunk_type) {
            if !quiet {
                println!("(skipping)");
            }

            f.seek(SeekFrom::Current(seek_size as i64))
             .map_err(convert_io_error)?;
        } else {
            if !quiet {
                let filename = format!(
                    "{:04}-{}.{}",
                    index,
                    chunk_offset,
                    // We know this is safe because of convert_fourcc
                    unsafe { std::str::from_utf8_unchecked(&chunk_type) }
                );
                print!("(dumping to: {}…", filename);
                let mut buffer = Vec::with_capacity(seek_size as usize);
                buffer.resize(seek_size as usize, 0);
                f.read_exact(&mut buffer[..]).map_err(convert_io_error)?;
                std::fs::write(filename, &buffer[..]).map_err(convert_io_error)?;
                println!(" done!)");
            }
        }

        offset += seek_size;
        index += 1;
    }

    println!("Finished reading file without problems!");

    Ok(())
}
