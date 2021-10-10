#!/bin/bash

set -eu

for sndS_file in *.sndS
do
    sndS_chunk_index="${sndS_file%%-*}" # turn XXXX-YYYYYY.sndS into just XXXX
    sndS_chunk_index=$((10#$sndS_chunk_index)) # strip leading zeroes (no octal)
    sndH_chunk_index="$(($sndS_chunk_index-1))" # sndH chunk index is one less
    sndH_chunk_index=$(printf '%04s' $sndH_chunk_index) # pad XX to 00XX
    sndH_file=($sndH_chunk_index-*-sndH.txt)
    echo $sndS_file '<=' $sndH_file
    sndH_args=`cat $sndH_file`
    echo ffmpeg $sndH_args -i "$sndS_file" "$sndS_file"_"$sndH_file"_.wav
    ffmpeg $sndH_args -i "$sndS_file" "$sndS_file"_"$sndH_file"_.wav
done
