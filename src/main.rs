use std::collections::HashSet;
use std::convert::TryInto;
use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str::FromStr;

fn print_usage() -> Result<(), String> {
    eprint!(
        "\
dream-sparer: RIFX file reader {} by hikari_no_yume. Copyright 2021.
MIT licensed.

Usage:
    dream-sparer path/to/rifx/file

With no other arguments passed, dream-sparer will just print a list of chunks
found in the file.

Optional arguments:

  --help            Display this help.
  --quiet-all=TYPE  Don't print anything for chunk type TYPE.
                    You can specify multiple types by repeating the argument.
  --dump=INDEX      When encountering the chunk with the index INDEX, dump it to
                    a file. The filename will use the format: INDEX_OFFSET.TYPE
                    You can specify multiple indices by repeating the argument.
  --dump-all=TYPE   When encountering a chunk of type TYPE, dump it to a file.
                    The files will be named the same way as for --dump.
                    You can specify multiple indices by repeating the argument.
",
        env!("CARGO_PKG_VERSION")
    );
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
    let mut dump_indices: HashSet<u32> = HashSet::new();
    for arg in &args[1..] {
        if arg == "--help" {
            return print_usage();
        } else if let Some(fourcc) = arg.strip_prefix("--quiet-all=") {
            quiet_fourccs.insert(convert_fourcc(fourcc)?);
        } else if let Some(fourcc) = arg.strip_prefix("--dump-all=") {
            dump_fourccs.insert(convert_fourcc(fourcc)?);
        } else if let Some(index) = arg.strip_prefix("--dump=") {
            let index = u32::from_str(index).map_err(|e| e.to_string())?;
            dump_indices.insert(index);
        } else if arg.starts_with("--") {
            return Err(format!("Unknown argument: '{}'", arg));
        } else {
            match filename {
                Some(_) => {
                    return Err(format!("Only one filename can be specified."));
                }
                None => {
                    filename = Some(arg);
                }
            }
        }
    }

    if let Some(filename) = filename {
        let mut file = File::open(filename).map_err(convert_io_error)?;

        read_riff_file(&mut file, &quiet_fourccs, &dump_fourccs, &dump_indices)
    } else {
        Err(format!("No filename was specified."))
    }
}

fn format_fourcc(f: FourCC) -> String {
    if f.is_ascii() {
        format!("'{}'", unsafe { std::str::from_utf8_unchecked(&f) })
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
    Ok(u32::from_le(unsafe {
        std::mem::transmute::<[u8; 4], u32>(buffer)
    }))
}

fn read_riff_file(
    f: &mut File,
    quiet_fourccs: &HashSet<FourCC>,
    dump_fourccs: &HashSet<FourCC>,
    dump_indices: &HashSet<u32>,
) -> Result<(), String> {
    let file_type = read_fourcc(f, false)?;
    print!(
        "File's magic number/FourCC is {}: ",
        format_fourcc(file_type)
    );
    if file_type == XFIR {
        println!("Little-endian RIFX file.");
    } else {
        println!("Unknown.");
        return Err(format!("This format is not supported yet."));
    }

    let file_size = read_u32_le(f)?;
    println!("File size according to RIFF header: {} bytes", file_size);

    let file_kind = read_fourcc(f, true)?;
    println!(
        "File kind according to RIFF header: {}",
        format_fourcc(file_kind)
    );

    let mut offset: u32 = 12;
    let mut index: u32 = 0;
    while offset < file_size {
        let chunk_type = read_fourcc(f, true)?;
        let chunk_size = read_u32_le(f)?;
        let chunk_offset = offset;
        let chunk_index = index;
        offset += 8;

        let quiet = quiet_fourccs.contains(&chunk_type);
        let dump = dump_fourccs.contains(&chunk_type) || dump_indices.contains(&chunk_index);
        if !quiet {
            println!(
                "Chunk #{} of type {}, size {} bytes at offset {} bytes",
                chunk_index,
                format_fourcc(chunk_type),
                chunk_size,
                chunk_offset
            );
        }

        // RIFF pads chunk sizes to be 2-byte-aligned (the era of “DWORDs”…)
        let seek_size = chunk_size + (chunk_size & 1);

        if !dump {
            if !quiet {
                println!("(skipping)");
            }

            f.seek(SeekFrom::Current(seek_size as i64))
                .map_err(convert_io_error)?;
        } else {
            if !quiet {
                let filename = format!(
                    "{:04}-{}.{}",
                    chunk_index,
                    chunk_offset,
                    std::str::from_utf8(&chunk_type).map_err(|e| e.to_string())?
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
