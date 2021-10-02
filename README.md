# dream-sparer

Simple tool for extracting chunks from little-endian RIFX files.

I wrote this specifically because I wanted to extract the audio files from a certain Macromedia Director game, and apparently Director files use RIFX.

It doesn't support anything other than little-endian RIFX currently, but if you would like me to add support for some other variation of RIFF, or some feature related to that, feel free to contact me — I might be interested in helping!

## What's RIFX?

RIFF but the FourCC codes have the wrong endianness. Yeah, really…

## Building

This is a simple Rust project. It uses stable Rust so `cargo build` should be enough.

## Features

```
dream-sparer: RIFX file reader 0.1.1 by hikari_no_yume. Copyright 2021.
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
```

The most useful thing is `--dump`. If you know what kind of chunks contain the stuff you're after, you can use this to extract them and, hopefully, you may be able to do something useful with them. In the case of the game I was interested in, `--dump=sndS` was enough to extract raw PCM sound samples, which I could then throw at Audacity or ffmpeg.
