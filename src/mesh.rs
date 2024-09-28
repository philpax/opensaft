#[derive(Clone, Default)]
pub struct TriangleMesh {
    pub indices: Vec<u32>,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 3]>,
}

impl TriangleMesh {
    /// Convert a triangle mesh to an OBJ file
    pub fn to_obj(&self) -> String {
        use std::fmt::Write as FmtWrite;

        let mesh = self;

        let mut s = String::new();
        writeln!(&mut s, "# Generated by saft-ext library").unwrap();

        // Adding vertex colors after vertex positions is a non-standard extension,
        // but a common one:
        writeln!(&mut s, "\n# Vertex positions and colors:").unwrap();
        assert_eq!(mesh.positions.len(), mesh.colors.len());
        for (p, c) in mesh.positions.iter().zip(&mesh.colors) {
            writeln!(s, "v {} {} {} {} {} {}", p[0], p[1], p[2], c[0], c[1], c[2]).unwrap();
        }

        writeln!(&mut s, "\n# Vertex normals:").unwrap();
        assert_eq!(mesh.positions.len(), mesh.normals.len());
        for n in &mesh.normals {
            writeln!(&mut s, "vn {} {} {}", n[0], n[1], n[2]).unwrap();
        }

        writeln!(&mut s, "\n# Triangle faces:").unwrap();
        assert_eq!(mesh.indices.len() % 3, 0);
        for t in mesh.indices.chunks(3) {
            // OBJ uses 1-based indexing, like some sort of cave man
            writeln!(&mut s, "f {} {} {}", t[0] + 1, t[1] + 1, t[2] + 1).unwrap();
        }

        writeln!(&mut s, "\n# End of obj file.").unwrap();

        s
    }
}