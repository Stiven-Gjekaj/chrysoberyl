#!/usr/bin/env python3
"""Write the two PDF fixtures in tests/golden/formats/pdf.

Each file has two pages.
Page 1 holds text, a filled rectangle and a curve.
Page 2 holds text only.
The candidate gives the rectangle on page 1 a new colour.
The engine compares near-identical pairs only, so the change is small.
Page 2 is the same in the two files.

The script writes the cross-reference table from the real byte offsets.
It uses only the standard library.
It holds no image, so the image path of the rasterizer does not run.
"""

import pathlib
import sys


def page_one(colour: str) -> bytes:
    return (
        "BT /F1 24 Tf 40 230 Td (Chrysoberyl) Tj ET\n"
        "BT /F1 14 Tf 40 200 Td (The first page) Tj ET\n"
        f"{colour} rg\n"
        "40 60 80 60 re f\n"
        "0 0 0 RG 2 w\n"
        "200 60 m 230 140 260 20 290 100 c S\n"
    ).encode("ascii")


def page_two() -> bytes:
    return b"BT /F1 18 Tf 40 150 Td (Second page) Tj ET\n"


def build(colour: str) -> bytes:
    contents = [page_one(colour), page_two()]
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 320 280] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Contents 6 0 R >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 320 280] "
        b"/Resources << /Font << /F1 5 0 R >> >> /Contents 7 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ]
    for stream in contents:
        objects.append(
            b"<< /Length %d >>\nstream\n" % len(stream) + stream + b"endstream"
        )

    out = bytearray(b"%PDF-1.4\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(out))
        out += b"%d 0 obj\n" % number + body + b"\nendobj\n"
    xref = len(out)
    out += b"xref\n0 %d\n" % (len(objects) + 1)
    out += b"0000000000 65535 f \n"
    for offset in offsets:
        out += b"%010d 00000 n \n" % offset
    out += b"trailer\n<< /Size %d /Root 1 0 R >>\n" % (len(objects) + 1)
    out += b"startxref\n%d\n%%%%EOF\n" % xref
    return bytes(out)


def main() -> None:
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tests/golden/formats/pdf")
    root.mkdir(parents=True, exist_ok=True)
    (root / "base.pdf").write_bytes(build("0.1 0.4 0.8"))
    (root / "candidate.pdf").write_bytes(build("0.8 0.2 0.1"))


if __name__ == "__main__":
    main()
