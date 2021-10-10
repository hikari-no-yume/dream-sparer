# dream-sparer

Simple tool for extracting chunks from RIFX files.

I wrote this specifically because I wanted to extract the audio files from a certain Macromedia Director game, and apparently Director files use RIFX. (See the section right at the end for more details.) Naturally, the Windows ones are little-endian and the Mac OS ones are big-endian.

This doesn't support anything other than little- and big-endian RIFX currently, but if you would like me to add support for e.g. normal RIFF, or something else with some connection to RIFF, feel free to contact me — I might be interested in helping!

## What's RIFX?

A big-endian version of RIFF. But there's also the little-endian version of RIFX (XFIR), which only differs from RIFF in that the FourCC codes are backwards.

## Building

This is a simple Rust project. It uses stable Rust so `cargo build` should be enough.

## Features

```
dream-sparer: RIFX file reader 0.2.1 by hikari_no_yume. Copyright 2021.
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
```

The most useful thing is `--dump`. If you know what kind of chunks contain the stuff you're after, you can use this to extract them and, hopefully, you may be able to do something useful with them.

### Special feature: `--translate-sndH`

Most of the features of this tool should be generic and work for any RIFX file. `--translate-sndH` is something more specific, however: it tries to parse the chunk type (`sndH`) used by Macromedia Director (at least version 8.5) for PCM sound clip headers.

The basic usage for this case is:

```
./dream-sparer some-director-file.cst --quiet-all='snd ' --dump-all=sndS --translate-sndH
```

This will dump the raw PCM sound samples to `.sndS` files, and create `.txt` files containing what should be valid arguments to `ffmpeg` for decoding the `.sndS` files correctly. Here's an example of what such a `.txt` file may contain:

```
-f u8 -ac 2 -ar 22050
```

Note that the index of the `sndH` (header) chunk is going to be one less than the index of the `sndS` (raw PCM sound samples) chunk. So you'll need to match e.g. `0020-….sndS` with `0019-…-sndH.txt`, and so on.

You can then hopefully use those arguments to decode those `.sndS` files in this kind of way:

```
ffmpeg -f u8 -ac 2 -ar 22050 -i 0020-something.sndS decoded-0020.wav
```

To automate this process, I have included a small Bash script in the repository (`convert-sndS-sndH-to-wav.sh`) that will do this for you for all the dumped files. It's the least robust component of this project, though, and I've no idea if you can use it on Windows, for example.

Note that `sndS`/`sndH` chunks are not the only way Macromedia Director can store audio. For instance, when I tried to import an IMA ADPCM `.wav` file, I got an `ediM` chunk instead. For those, just `--dump-all=ediM` might be enough to get useful files out (it turned out to be a RIFF WAVE, aka `.wav`, file). I'm sure the same kind of thing might apply to some other audio or media types.
