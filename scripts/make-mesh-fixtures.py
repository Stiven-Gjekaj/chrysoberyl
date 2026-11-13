#!/usr/bin/env python3
"""Write the two mesh fixtures in tests/golden/formats/mesh.

Each file is a Wavefront OBJ with three boxes:

- a base block, 2 by 1 by 1,
- a top block, 1 by 0.5 by 1, on the base block,
- a small cube inside the base block. No view sees it, so its 12
  triangles are the measured blind spot.

The candidate moves the top block 0.2 to the right. The bounding box of
the mesh does not change, so the scale of each view does not change.
"""

import pathlib
import sys


def box(low, high):
    (x0, y0, z0), (x1, y1, z1) = low, high
    corners = [
        (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
        (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1),
    ]
    faces = [
        (0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4),
        (3, 7, 6, 2), (0, 4, 7, 3), (1, 2, 6, 5),
    ]
    return corners, faces


def write(path, shift):
    boxes = [
        box((0, 0, 0), (2, 1, 1)),
        box((0 + shift, 1, 0), (1 + shift, 1.5, 1)),
        box((0.8, 0.4, 0.4), (1.2, 0.6, 0.6)),
    ]
    lines = ["# Chrysoberyl mesh fixture"]
    offset = 0
    for corners, faces in boxes:
        lines += [f"v {x:g} {y:g} {z:g}" for x, y, z in corners]
        lines += ["f " + " ".join(str(offset + i + 1) for i in face) for face in faces]
        offset += len(corners)
    path.write_text("\n".join(lines) + "\n")


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "tests/golden/formats/mesh")
    root.mkdir(parents=True, exist_ok=True)
    write(root / "base.obj", 0.0)
    write(root / "candidate.obj", 0.2)


if __name__ == "__main__":
    main()
