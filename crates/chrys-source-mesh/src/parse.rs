//! Readers for Wavefront OBJ and for STL, ASCII and binary.
//!
//! Each reader gives a list of triangles. A polygon in an OBJ file
//! becomes a fan of triangles from its first vertex.
//! `str::parse::<f32>` rounds correctly, so a number in a text file gives
//! the same `f32` on each machine.

/// One triangle: three points.
pub type Triangle = [[f32; 3]; 3];

/// The reason that a mesh file does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The format of a mesh file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshFormat {
    /// Wavefront OBJ.
    Obj,
    /// STL in text.
    AsciiStl,
    /// STL in binary.
    BinaryStl,
}

/// The first bytes of the image formats that the raster adapters read. A
/// file that starts with one of them is never a mesh.
const IMAGE_MAGIC: &[&[u8]] = &[
    b"\x89PNG",
    b"GIF8",
    b"\xff\xd8",
    b"RIFF",
    b"II*\0",
    b"MM\0*",
    b"BM",
    b"%PDF",
    b"DKIF",
];

/// Find the format of a file from its content only.
///
/// `bytes` is the start of the file, at least 84 bytes when the file is
/// that long. `length` is the length of the whole file. A binary STL has
/// no signature, so it is found when its length agrees with the triangle
/// count in its header.
pub fn sniff(bytes: &[u8], length: u64) -> Option<MeshFormat> {
    if IMAGE_MAGIC.iter().any(|magic| bytes.starts_with(magic)) {
        return None;
    }
    let head = &bytes[..bytes.len().min(4096)];
    if let Ok(text) = std::str::from_utf8(head) {
        let trimmed = text.trim_start();
        if trimmed.starts_with("solid") && text.contains("facet") {
            return Some(MeshFormat::AsciiStl);
        }
        let lines = || text.lines().map(str::trim_start);
        if lines().any(|line| line.starts_with("v ")) && text.contains("\nf ") {
            return Some(MeshFormat::Obj);
        }
    }
    if bytes.len() >= 84 {
        let count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as u64;
        if count > 0 && length == 84 + 50 * count {
            return Some(MeshFormat::BinaryStl);
        }
    }
    None
}

fn number(token: Option<&str>, line: usize) -> Result<f32, ParseError> {
    let token = token.ok_or_else(|| ParseError(format!("line {line}: a number is missing")))?;
    let value: f32 = token
        .parse()
        .map_err(|_| ParseError(format!("line {line}: {token:?} is not a number")))?;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ParseError(format!("line {line}: {token:?} is not finite")))
    }
}

/// Parse an OBJ file.
pub fn obj(text: &str) -> Result<Vec<Triangle>, ParseError> {
    let mut vertices: Vec<[f32; 3]> = Vec::new();
    let mut triangles = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let mut tokens = line.split_whitespace();
        match tokens.next() {
            Some("v") => vertices.push([
                number(tokens.next(), line_number)?,
                number(tokens.next(), line_number)?,
                number(tokens.next(), line_number)?,
            ]),
            Some("f") => {
                let corners = tokens
                    .map(|token| vertex_index(token, vertices.len(), line_number))
                    .collect::<Result<Vec<usize>, ParseError>>()?;
                if corners.len() < 3 {
                    return Err(ParseError(format!(
                        "line {line_number}: a face needs three vertices"
                    )));
                }
                for pair in corners[1..].windows(2) {
                    triangles.push([vertices[corners[0]], vertices[pair[0]], vertices[pair[1]]]);
                }
            }
            _ => {}
        }
    }
    Ok(triangles)
}

/// Read one corner of an OBJ face: `v`, `v/vt`, `v//vn` or `v/vt/vn`.
/// A negative index counts back from the last vertex.
fn vertex_index(token: &str, count: usize, line: usize) -> Result<usize, ParseError> {
    let first = token.split('/').next().unwrap_or("");
    let value: i64 = first
        .parse()
        .map_err(|_| ParseError(format!("line {line}: {token:?} is not a vertex index")))?;
    let resolved = if value > 0 {
        value - 1
    } else {
        count as i64 + value
    };
    if value == 0 || resolved < 0 || resolved >= count as i64 {
        return Err(ParseError(format!(
            "line {line}: the vertex index {value} is outside 1 to {count}"
        )));
    }
    Ok(resolved as usize)
}

/// Parse an ASCII STL file.
pub fn ascii_stl(text: &str) -> Result<Vec<Triangle>, ParseError> {
    let mut points = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let mut tokens = line.split_whitespace();
        if tokens.next() == Some("vertex") {
            points.push([
                number(tokens.next(), index + 1)?,
                number(tokens.next(), index + 1)?,
                number(tokens.next(), index + 1)?,
            ]);
        }
    }
    if points.len() % 3 != 0 {
        return Err(ParseError(
            "the number of vertices is not a multiple of three".to_string(),
        ));
    }
    Ok(points.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

/// Parse a binary STL file.
pub fn binary_stl(bytes: &[u8]) -> Result<Vec<Triangle>, ParseError> {
    let count = bytes
        .get(80..84)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        .ok_or_else(|| ParseError("the file is shorter than the STL header".to_string()))?;
    let body = &bytes[84..];
    if body.len() != count * 50 {
        return Err(ParseError(format!(
            "the header declares {count} triangles, and the file holds {} bytes of them",
            body.len()
        )));
    }
    body.chunks_exact(50)
        .map(|record| {
            let value = |at: usize| {
                let v = f32::from_le_bytes([
                    record[at],
                    record[at + 1],
                    record[at + 2],
                    record[at + 3],
                ]);
                if v.is_finite() {
                    Ok(v)
                } else {
                    Err(ParseError("a vertex is not finite".to_string()))
                }
            };
            let point = |at: usize| -> Result<[f32; 3], ParseError> {
                Ok([value(at)?, value(at + 4)?, value(at + 8)?])
            };
            // Bytes 0 to 11 hold the normal, which this reader does not use.
            Ok([point(12)?, point(24)?, point(36)?])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quad_becomes_two_triangles() {
        let text = "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1/1/1 2//2 3 -1\n";
        let triangles = obj(text).expect("parse");
        assert_eq!(triangles.len(), 2);
        assert_eq!(
            triangles[1],
            [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]]
        );
    }

    #[test]
    fn a_vertex_index_outside_the_list_is_refused() {
        assert!(obj("v 0 0 0\nf 1 2 3\n").is_err());
    }

    #[test]
    fn binary_and_ascii_stl_give_the_same_triangle() {
        let mut bytes = vec![0u8; 80];
        bytes.extend_from_slice(&1u32.to_le_bytes());
        for value in [
            0.0f32, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&[0, 0]);
        let text = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        assert_eq!(
            sniff(&bytes, bytes.len() as u64),
            Some(MeshFormat::BinaryStl)
        );
        assert_eq!(
            sniff(text.as_bytes(), text.len() as u64),
            Some(MeshFormat::AsciiStl)
        );
        assert_eq!(
            binary_stl(&bytes).expect("binary"),
            ascii_stl(text).expect("ascii")
        );
    }

    #[test]
    fn an_image_is_never_a_mesh() {
        let mut png = b"\x89PNG".to_vec();
        png.resize(134, 0);
        png[80] = 1;
        assert_eq!(sniff(&png, png.len() as u64), None);
    }
}
