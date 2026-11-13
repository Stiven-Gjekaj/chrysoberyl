#!/usr/bin/env bash
# Write the AV1 fixtures in tests/golden/formats/av1, and the reference
# hashes of the decoded frames.
#
# The encoder is SVT-AV1 through ffmpeg. Each frame is a key frame, so a
# change in one frame cannot change the bits of another frame.
#
# The reference hashes come from libdav1d through ffmpeg. That decoder is
# not the decoder in this repository, so the conformance test compares two
# independent decoders.
#
# Files:
#   base.ivf       8 frames at 10 fps, BT.709, limited range.
#   candidate.ivf  the same, with a box in a new colour on frames 4 to 7.
#   double.ivf     the frames of base.ivf at 20 fps, each frame two times.
#                  It shows the same picture as base.ivf at each time.
#   full601.ivf    2 frames, BT.601, full range.
#   untagged.ivf   1 frame with no colour tags. The adapter refuses it.
set -euo pipefail

out="${1:-tests/golden/formats/av1}"
mkdir -p "$out"

src="testsrc2=size=64x48:rate=10:duration=0.8"
common=(-hide_banner -loglevel error -y -c:v libsvtav1 -crf 12 -svtav1-params keyint=1 -pix_fmt yuv420p)
tags709=(-colorspace bt709 -color_primaries bt709 -color_trc bt709 -color_range tv)

ffmpeg -f lavfi -i "$src" "${common[@]}" "${tags709[@]}" "$out/base.ivf"
ffmpeg -f lavfi -i "$src" \
  -vf "drawbox=x=8:y=8:w=16:h=12:color=red@1:t=fill:enable='gte(n,4)'" \
  "${common[@]}" "${tags709[@]}" "$out/candidate.ivf"
ffmpeg -f lavfi -i "$src" -vf "fps=20" "${common[@]}" "${tags709[@]}" "$out/double.ivf"
ffmpeg -f lavfi -i "testsrc2=size=64x48:rate=10:duration=0.2" "${common[@]}" \
  -colorspace smpte170m -color_primaries smpte170m -color_trc smpte170m -color_range pc \
  "$out/full601.ivf"

ffmpeg -f lavfi -i "testsrc2=size=64x48:rate=10:duration=0.1" "${common[@]}" "$out/untagged.ivf"

: > "$out/REFERENCE.sha256"
for file in base candidate double full601 untagged; do
  ffmpeg -hide_banner -loglevel error -c:v libdav1d -i "$out/$file.ivf" \
    -f framehash -hash sha256 - |
    awk -v name="$file.ivf" -F, '!/^#/ { gsub(/ /, ""); print name, $3, $6 }' \
    >> "$out/REFERENCE.sha256"
done
