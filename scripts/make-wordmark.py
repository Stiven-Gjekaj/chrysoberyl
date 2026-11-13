#!/usr/bin/env python3
"""Write assets/brand/wordmark.svg, the banner at the top of the README.

The word is a set of pixel paths, not text. GitHub removes external fonts,
so a path renders the same everywhere. The script uses only the standard
library.

The colours run from chrysoberyl, a yellow green, to alexandrite, the same
mineral that shows purple and red under a different light.
"""

import pathlib
import sys

PIXEL = 7

# A 5 by 9 pixel font: two rows above the x-height, five rows of
# x-height, and two rows below for a descender.
GLYPHS = {
    "c": [".....", ".....", ".###.", "#....", "#....", "#....", ".###.", ".....", "....."],
    "h": ["#....", "#....", "####.", "#...#", "#...#", "#...#", "#...#", ".....", "....."],
    "r": [".....", ".....", "#.##.", "##...", "#....", "#....", "#....", ".....", "....."],
    "y": [".....", ".....", "#...#", "#...#", "#...#", "#...#", ".####", "....#", ".###."],
    "s": [".....", ".....", ".####", "#....", ".###.", "....#", "####.", ".....", "....."],
    "o": [".....", ".....", ".###.", "#...#", "#...#", "#...#", ".###.", ".....", "....."],
    "b": ["#....", "#....", "####.", "#...#", "#...#", "#...#", "####.", ".....", "....."],
    "e": [".....", ".....", ".###.", "#...#", "#####", "#....", ".###.", ".....", "....."],
    "l": ["##...", ".#...", ".#...", ".#...", ".#...", ".#...", ".##..", ".....", "....."],
}

# The gem: a cut stone, 15 by 15 cells. "D" is the dark body, "F" a facet,
# "R" the rim.
GEM = [
    "....RRRRRRR....",
    "...RFFFDDDDR...",
    "..RFFFDDDDDDR..",
    ".RFFDDDDDDDDDR.",
    "RFFDDDDDDDDDDDR",
    "RFDDDDDDDDDDDDR",
    "RFDDDDDDDDDDDDR",
    "RDDDDDDDDDDDDDR",
    "RDDDDDDDDDDDDFR",
    "RDDDDDDDDDDDDFR",
    "RDDDDDDDDDDDFFR",
    ".RDDDDDDDDDFFR.",
    "..RDDDDDDDFFR..",
    "...RDDDDDFFR...",
    "....RRRRRRR....",
]


def rects(rows, char, pixel, x0=0, y0=0):
    """Return path data for each run of `char` in `rows`."""
    parts = []
    for y, row in enumerate(rows):
        x = 0
        while x < len(row):
            if row[x] == char:
                start = x
                while x < len(row) and row[x] == char:
                    x += 1
                width = (x - start) * pixel
                parts.append(f"M{x0 + start * pixel} {y0 + y * pixel}h{width}v{pixel}h-{width}z")
            else:
                x += 1
    return "".join(parts)


def word_rows(word):
    rows = [""] * 9
    for letter in word:
        for i in range(9):
            rows[i] += GLYPHS[letter][i] + "."
    return [row[:-1] for row in rows]


CELL = 12
PITCH = 13
GRID_GAP = 10


def grid(x0, y0, changed):
    """Two 3 by 3 grids of cells. The candidate grid has one changed cell."""
    tones = ["#5f7a2a", "#8bc34a", "#26a69a", "#4f6f2a"]
    out = []
    second = x0 + 3 * PITCH + GRID_GAP
    for gx, is_candidate in ((x0, False), (second, True)):
        for row in range(3):
            for col in range(3):
                fill = tones[(row * 2 + col) % 4]
                if is_candidate and (row, col) == changed:
                    fill = "#ec407a"
                out.append(
                    f'<rect x="{gx + col * PITCH}" y="{y0 + row * PITCH}" '
                    f'width="{CELL}" height="{CELL}" fill="{fill}"/>'
                )
    return "\n      ".join(out)


def main():
    out = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "assets/brand/wordmark.svg")
    out.parent.mkdir(parents=True, exist_ok=True)

    word = rects(word_rows("chrysoberyl"), "#", PIXEL)
    word_width = (len("chrysoberyl") * 6 - 1) * PIXEL
    word_x = 160
    word_y = (180 - 9 * PIXEL) // 2 + 4

    gem_x, gem_y = 38, 37
    gem_side = 15 * PIXEL
    changed = (1, 2)
    grid_x = word_x + word_width + 22
    grid_y = (180 - 3 * PITCH) // 2
    box_x = grid_x + 3 * PITCH + GRID_GAP + changed[1] * PITCH - 3
    box_y = grid_y + changed[0] * PITCH - 3

    svg = f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 756 180" width="756" height="180" role="img" aria-label="Chrysoberyl">
  <title>Chrysoberyl</title>

  <!-- Made by scripts/make-wordmark.py. Edit the script, not this file. -->

  <defs>
    <!-- Chrysoberyl yellow green, then the colour change of alexandrite:
         teal, purple and red. The shift runs along the word. -->
    <linearGradient id="ink" x1="0" y1="0" x2="1" y2="0.35">
      <stop offset="0" stop-color="#d4e157"/>
      <stop offset="0.35" stop-color="#8bc34a"/>
      <stop offset="0.6" stop-color="#26a69a"/>
      <stop offset="0.82" stop-color="#ab47bc"/>
      <stop offset="1" stop-color="#ec407a"/>
    </linearGradient>

    <filter id="glow" x="-20%" y="-40%" width="140%" height="180%">
      <feGaussianBlur stdDeviation="6" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>

    <!-- The outline of the gem. The cat's-eye line stays inside it. -->
    <clipPath id="stone">
      <path d="{rects(GEM, "D", PIXEL, gem_x, gem_y)}{rects(GEM, "F", PIXEL, gem_x, gem_y)}"/>
    </clipPath>
  </defs>

  <!-- The panel is near black, so it reads on the light and on the dark
       GitHub theme. The border is yellow green at a low opacity. -->
  <rect width="756" height="180" rx="16" fill="#111111"/>
  <rect x="6" y="6" width="744" height="168" rx="11"
        fill="none" stroke="#d4e157" stroke-opacity="0.24" stroke-width="2"/>

  <g filter="url(#glow)">
    <!-- A pixel cat's-eye chrysoberyl: a dark green stone with one bright
         line across it. The line moves where SVG animation runs. -->
    <path d="{rects(GEM, "R", PIXEL, gem_x, gem_y)}" fill="#5f7a2a"/>
    <path d="{rects(GEM, "D", PIXEL, gem_x, gem_y)}" fill="#2f4a1f"/>
    <path d="{rects(GEM, "F", PIXEL, gem_x, gem_y)}" fill="#4f6f2a"/>
    <g clip-path="url(#stone)">
      <rect x="{gem_x + 7 * PIXEL}" y="{gem_y}" width="{PIXEL}" height="{gem_side}" fill="#f4f9d0" opacity="0.9">
        <animate attributeName="x" values="{gem_x + 4 * PIXEL};{gem_x + 10 * PIXEL};{gem_x + 4 * PIXEL}" dur="3.2s" repeatCount="indefinite"/>
      </rect>
    </g>

    <path d="{word}" fill="url(#ink)" transform="translate({word_x} {word_y})"/>

    <!-- A base and a candidate. One cell changed, and a box names it. -->
    <g>
      {grid(grid_x, grid_y, changed)}
    </g>
    <rect x="{box_x}" y="{box_y}" width="{CELL + 6}" height="{CELL + 6}" fill="none" stroke="#f4f9d0" stroke-width="2">
      <animate attributeName="opacity" values="1;0.2;1" dur="1.4s" repeatCount="indefinite"/>
    </rect>
  </g>
</svg>
'''
    out.write_text(svg)


if __name__ == "__main__":
    main()
