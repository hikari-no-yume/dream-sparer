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
  --translate-sndH  Not a generic RIFX option: specific to Macromedia Director.
                    Tries to decode sound clip headers ('sndH') into format
                    arguments understood by FFMPEG. One file is created for each
                    chunk, like --dump-all.
                    Supports 8-bit unsigned, and 16-, 24- and 32-bit signed PCM.
                    This tries to be generous with what it attempts to translate
                    and does not guarantee the resulting files are correct, but
                    it does output warnings when things don't look right.
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
    let mut translate_sndh: bool = false;
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
        } else if arg == "--translate-sndH" {
            translate_sndh = true;
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

        read_riff_file(
            &mut file,
            &quiet_fourccs,
            &dump_fourccs,
            &dump_indices,
            translate_sndh,
        )
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
const RIFX: FourCC = [b'R', b'I', b'F', b'X'];
#[allow(non_upper_case_globals)]
const sndH: FourCC = [b's', b'n', b'd', b'H'];

fn read_fourcc(f: &mut File, byteswap: bool) -> Result<FourCC, String> {
    let mut buffer = [0u8; 4];
    f.read_exact(&mut buffer).map_err(convert_io_error)?;
    if byteswap {
        buffer.reverse();
    }
    Ok(buffer)
}

fn read_u32(f: &mut File, little_endian: bool) -> Result<u32, String> {
    let mut buffer = [0u8; 4];
    f.read_exact(&mut buffer).map_err(convert_io_error)?;
    Ok(if little_endian {
        u32::from_le_bytes(buffer)
    } else {
        u32::from_be_bytes(buffer)
    })
}

fn read_riff_file(
    f: &mut File,
    quiet_fourccs: &HashSet<FourCC>,
    dump_fourccs: &HashSet<FourCC>,
    dump_indices: &HashSet<u32>,
    translate_sndh: bool,
) -> Result<(), String> {
    let file_type = read_fourcc(f, false)?;
    print!(
        "File's magic number/FourCC is {}: ",
        format_fourcc(file_type)
    );

    let little_endian = if file_type == XFIR {
        println!("Little-endian RIFX file.");
        true
    } else if file_type == RIFX {
        println!("Big-endian RIFX file.");
        false
    } else {
        return Err(format!("This format is not supported yet."));
    };

    let file_size = read_u32(f, little_endian)?;
    println!("File size according to RIFF header: {} bytes", file_size);

    let file_kind = read_fourcc(f, little_endian)?;
    println!(
        "File kind according to RIFF header: {}",
        format_fourcc(file_kind)
    );

    let mut offset: u32 = 12;
    let mut index: u32 = 0;
    while offset < file_size {
        let chunk_type = read_fourcc(f, little_endian)?;
        let chunk_size = read_u32(f, little_endian)?;
        let chunk_offset = offset;
        let chunk_index = index;
        offset += 8;

        let quiet = quiet_fourccs.contains(&chunk_type);
        let dump = dump_fourccs.contains(&chunk_type) || dump_indices.contains(&chunk_index);
        let translate = translate_sndh && chunk_type == sndH;
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

        if !dump && !translate {
            if !quiet {
                println!("(skipping)");
            }

            f.seek(SeekFrom::Current(seek_size as i64))
                .map_err(convert_io_error)?;
        } else {
            let mut buffer = Vec::with_capacity(seek_size as usize);
            buffer.resize(seek_size as usize, 0);
            f.read_exact(&mut buffer[..]).map_err(convert_io_error)?;
            if dump {
                let filename = format!(
                    "{:04}-{}.{}",
                    chunk_index,
                    chunk_offset,
                    std::str::from_utf8(&chunk_type).map_err(|e| e.to_string())?
                );
                if !quiet {
                    print!("(dumping to: {}…", filename);
                }
                std::fs::write(filename, &buffer[..]).map_err(convert_io_error)?;
                if !quiet {
                    println!(" done!)");
                }
            }
            if translate {
                do_translate_sndh(&buffer, quiet, chunk_index, chunk_offset)?
            }
        }

        offset += seek_size;
        index += 1;
    }

    println!("Finished reading file without problems!");

    Ok(())
}

macro_rules! tl_asserts {
    (
        $index: ident; $offset: ident; $buf: ident;
        ( $($var: ident),+ );
        $($expr: expr),+
    ) => {
        let mut failed = false;
        $(if !$expr {
            eprintln!(
                "sndH #{} (offset {}) failed assertion: {}",
                $index, $offset, stringify!($expr),
            );
            failed = true;
        });+
        if failed {
            eprintln!("{:?}", $buf);
            $(eprintln!("{}: {}", stringify!($var), $var));+
        }
    }
}

fn do_translate_sndh(
    buffer: &[u8],
    quiet: bool,
    chunk_index: u32,
    chunk_offset: u32,
) -> Result<(), String> {
    if buffer.len() != 100 {
        eprintln!(
            "sndH #{} (offset {}) is not 100 bytes long; ignoring",
            chunk_index, chunk_offset,
        );
        return Ok(());
    };
    let buffer = unsafe {
        let mut buffer = std::ptr::read_unaligned(buffer.as_ptr() as *const [u32; 25]);
        for i in 0..buffer.len() {
            buffer[i] = u32::from_be(buffer[i]) // Yes, not LE!
        }
        buffer
    };

    // These seem to be something like a GUID; they have no obvious meaning.
    let magic_numbers = &buffer[21..];
    // This one is seen in the game this app was written for
    if magic_numbers != [0x6a528ef2, 0x081011d0, 0xb28a0005, 0x02e85810] &&
       // This is what a freshly created Director 8.5 file uses
       buffer[21] != 0x6a5293a2
    {
        eprintln!(
            "sndH #{} (offset {}) has unexpected magic numbers: {:x?}",
            chunk_index, chunk_offset, magic_numbers
        );
    }

    {
        let zeros = (buffer[0], &buffer[2..8], &buffer[13..17]);
        if zeros != (0, &[0; 6], &[0; 4]) {
            eprintln!(
                "sndH #{} (offset {}) has unexpected non-zero values: {:?}",
                chunk_index, chunk_offset, zeros
            )
        }
    };

    let byte_count1 = buffer[1]; // Should match sndS chunk size
    let byte_count2 = buffer[8]; // Never seen these not match
    let pcm_frame_count1 = buffer[9];
    let pcm_frame_count2 = buffer[10]; // Never seen these not match
    let pcm_frames_per_second = buffer[11]; // e.g. 22050 for 22050Hz
    let bytes_per_second = buffer[12];
    let bit_depth = buffer[17]; // e.g. 16 for 16-bit
    let bytes_per_sample = buffer[18]; // e.g. 2 for 16-bit
    let channel_count = buffer[19]; // e.g. 2 for stereo
    let bytes_per_frame = buffer[20]; // e.g. 4 for stereo 16-bit

    tl_asserts!(
        chunk_index; chunk_offset; buffer;

        (
            byte_count1,
            byte_count2,
            pcm_frame_count1,
            pcm_frame_count2,
            pcm_frames_per_second,
            bytes_per_second,
            bit_depth,
            bytes_per_sample,
            channel_count,
            bytes_per_frame
        );

        byte_count1 == byte_count2,
        pcm_frame_count1 == pcm_frame_count2,
        bytes_per_second == pcm_frames_per_second * bytes_per_frame,
        bit_depth % 8 == 0,
        bytes_per_sample >= (bit_depth / 8),
        bytes_per_frame == bytes_per_sample * channel_count
    );

    // Note the inconsistent endianness! I don't know why this is.
    // Note also that the bit depth alone seems to determine the format.
    // Director 8.5 doesn't seem to import  µ-law, A-law or float WAV files,
    // so these are the only formats I know about.
    // Importing an IMA ADPCM file resulted in an 'ediM' chunk instead.
    let format = match bit_depth {
        8 => "u8",
        16 => "s16be", // Observed in the wild and contrived Director 8.5 file
        24 => "s24le", // Seen only in contrived Director 8.5 testing file
        32 => "s32le", // ditto
        _ => {
            eprintln!(
                "sndH #{} (offset {}) has unexpected bit-depth {}; ignoring",
                chunk_index, chunk_offset, bit_depth
            );
            return Ok(());
        }
    };
    let ffmpeg_args = format!(
        "-f {} -ac {} -ar {}",
        format, channel_count, pcm_frames_per_second,
    );

    let filename = format!("{:04}-{}-sndH.txt", chunk_index, chunk_offset,);

    if !quiet {
        print!("(writing translated sndH to: {}…", filename);
    }
    std::fs::write(filename, ffmpeg_args.as_bytes()).map_err(convert_io_error)?;
    if !quiet {
        println!(" done!)");
    }

    Ok(())
}
