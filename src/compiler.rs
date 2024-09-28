use super::graph::Graph;
use super::graph::Node;
use super::graph::NodeId;
use super::program::Program;
use super::Material;
use glam::Quat;
use glam::Vec2;
use glam::Vec3;
use glam::Vec4;
use opensaft_sdf::Opcode;

#[derive(thiserror::Error, Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Invalid program: {0}")]
    BadProgram(&'static str),

    #[error("Too few constants in program")]
    BadConstants,

    #[error("Unbalanced stack when interpreting program")]
    BadStack,

    #[error("NaN encountered in distance field")]
    EvaluatedToNaN,
}

pub struct ConstantReader<'a> {
    constants: &'a [f32],
    offset: usize,
}

impl<'a> ConstantReader<'a> {
    pub fn new(constants: &'a [f32]) -> Self {
        Self {
            constants,
            offset: 0,
        }
    }

    pub fn skip(&mut self, count: usize) {
        self.offset += count;
    }

    pub fn at_end(&self) -> bool {
        self.offset == self.constants.len()
    }

    pub fn read_f32(&mut self) -> Result<f32, Error> {
        if let Some(&value) = self.constants.get(self.offset) {
            self.offset += 1;
            Ok(value)
        } else {
            Err(Error::BadConstants)
        }
    }

    pub fn read_vec2(&mut self) -> Result<Vec2, Error> {
        let x = self.read_f32()?;
        let y = self.read_f32()?;
        let value = Vec2::new(x, y);
        Ok(value)
    }

    pub fn read_vec3(&mut self) -> Result<Vec3, Error> {
        let x = self.read_f32()?;
        let y = self.read_f32()?;
        let z = self.read_f32()?;
        let value = Vec3::new(x, y, z);
        Ok(value)
    }

    pub fn read_vec4(&mut self) -> Result<Vec4, Error> {
        let x = self.read_f32()?;
        let y = self.read_f32()?;
        let z = self.read_f32()?;
        let w = self.read_f32()?;
        let value = Vec4::new(x, y, z, w);
        Ok(value)
    }

    pub fn read_quat(&mut self) -> Result<Quat, Error> {
        let x = self.read_f32()?;
        let y = self.read_f32()?;
        let z = self.read_f32()?;
        let w = self.read_f32()?;
        let value: Quat = Quat::from_xyzw(x, y, z, w);
        Ok(value)
    }
}

pub struct ConstantEditor<'a> {
    constants: &'a mut [f32],
    offset: usize,
}

impl<'a> ConstantEditor<'a> {
    pub fn new(constants: &'a mut [f32]) -> Self {
        Self {
            constants,
            offset: 0,
        }
    }

    pub fn skip(&mut self, count: usize) {
        self.offset += count;
    }

    // Specific skips for readability
    pub fn skip_f32(&mut self) {
        self.skip(1);
    }

    pub fn skip_vec3(&mut self) {
        self.skip(3);
    }

    pub fn at_end(&self) -> bool {
        self.offset == self.constants.len()
    }

    pub fn edit_f32(&mut self, editor: impl FnOnce(f32) -> f32) -> Result<(), Error> {
        if let Some(value) = self.constants.get_mut(self.offset) {
            *value = editor(*value);
            self.offset += 1;
            Ok(())
        } else {
            Err(Error::BadConstants)
        }
    }

    pub fn edit_vec3(&mut self, editor: impl FnOnce(Vec3) -> Vec3) -> Result<(), Error> {
        // TODO: I'm sure there's some much better way to write this.
        if let Some([x, y, z]) = self.constants.get_mut(self.offset..(self.offset + 3)) {
            let edited = editor(Vec3::new(*x, *y, *z));
            *x = edited.x;
            *y = edited.y;
            *z = edited.z;
            self.offset += 3;
            Ok(())
        } else {
            Err(Error::BadConstants)
        }
    }
}

fn compile_node(graph: &Graph, root: NodeId, ctx: &mut Program, path: &mut Vec<NodeId>) {
    assert!(!path.contains(&root), "Graph cannot contain cycles!");

    path.push(root);

    let node = graph.get(root).unwrap();

    // Interpreter functions divides by the smoothing constant.
    // To prevent NaNs an Infs from ending up in the sdf we clamp
    // the smoothing constant when compiling the program.
    const MIN_SMOOTHING: f32 = 0.0001;

    match node {
        Node::Plane(plane) => {
            ctx.opcodes.push(Opcode::Plane);
            ctx.constant_push_vec4(*plane);
        }
        Node::Sphere { center, radius } => {
            ctx.opcodes.push(Opcode::Sphere);
            ctx.constant_push_vec3(*center);
            ctx.constants.push(*radius);
        }
        Node::Capsule { points, radius } => {
            if points[0] == points[1] {
                ctx.opcodes.push(Opcode::Sphere);
                ctx.constant_push_vec3(points[0]);
            } else {
                ctx.opcodes.push(Opcode::Capsule);
                ctx.constant_push_vec3(points[0]);
                ctx.constant_push_vec3(points[1]);
            }
            ctx.constants.push(*radius);
        }
        Node::RoundedCylinder {
            cylinder_radius,
            half_height,
            rounding_radius,
        } => {
            ctx.opcodes.push(Opcode::RoundedCylinder);
            ctx.constants.push(*cylinder_radius);
            ctx.constants.push(*half_height);
            ctx.constants.push(*rounding_radius);
        }
        Node::TaperedCapsule { points, radii } => {
            ctx.opcodes.push(Opcode::TaperedCapsule);
            ctx.constant_push_vec3(points[0]);
            ctx.constants.push(radii[0]);
            ctx.constant_push_vec3(points[1]);
            ctx.constants.push(radii[1]);
        }
        Node::Cone { radius, height } => {
            ctx.opcodes.push(Opcode::Cone);
            ctx.constants.push(*radius);
            ctx.constants.push(*height);
        }
        Node::RoundedBox {
            half_size,
            rounding_radius,
        } => {
            ctx.opcodes.push(Opcode::RoundedBox);
            ctx.constant_push_vec3(*half_size);
            ctx.constants.push(*rounding_radius);
        }
        Node::Torus { big_r, small_r } => {
            ctx.opcodes.push(Opcode::Torus);
            ctx.constants.push(*big_r);
            ctx.constants.push(*small_r);
        }
        Node::TorusSector {
            big_r,
            small_r,
            sin_cos_half_angle,
        } => {
            ctx.opcodes.push(Opcode::TorusSector);
            ctx.constants.push(*big_r);
            ctx.constants.push(*small_r);
            ctx.constant_push_vec2(<[f32; 2]>::from(*sin_cos_half_angle));
        }
        Node::BiconvexLens {
            lower_sagitta,
            upper_sagitta,
            chord,
        } => {
            ctx.opcodes.push(Opcode::BiconvexLens);
            ctx.constants.push(*lower_sagitta);
            ctx.constants.push(*upper_sagitta);
            ctx.constants.push(*chord);
        }
        Node::Material { child, material } => {
            compile_node(graph, *child, ctx, path);
            ctx.opcodes.push(Opcode::Material);
            ctx.constant_push_vec3(material.rgb());
        }

        Node::Union { lhs, rhs } => {
            compile_node(graph, *lhs, ctx, path);
            compile_node(graph, *rhs, ctx, path);
            ctx.opcodes.push(Opcode::Union);
        }
        Node::UnionSmooth { lhs, rhs, size } => {
            compile_node(graph, *lhs, ctx, path);
            compile_node(graph, *rhs, ctx, path);
            ctx.opcodes.push(Opcode::UnionSmooth);
            ctx.constants.push(size.max(MIN_SMOOTHING));
        }
        Node::UnionMulti { children } => {
            for (idx, child) in children.iter().enumerate() {
                compile_node(graph, *child, ctx, path);
                if idx > 0 {
                    ctx.opcodes.push(Opcode::Union);
                }
            }
        }
        Node::UnionMultiSmooth { children, size } => {
            for (idx, child) in children.iter().enumerate() {
                compile_node(graph, *child, ctx, path);
                if idx > 0 {
                    ctx.opcodes.push(Opcode::UnionSmooth);
                    ctx.constants.push(size.max(MIN_SMOOTHING));
                }
            }
        }
        Node::Subtract { lhs, rhs } => {
            compile_node(graph, *lhs, ctx, path);
            compile_node(graph, *rhs, ctx, path);
            ctx.opcodes.push(Opcode::Subtract);
        }
        Node::SubtractSmooth { lhs, rhs, size } => {
            compile_node(graph, *lhs, ctx, path);
            compile_node(graph, *rhs, ctx, path);
            ctx.opcodes.push(Opcode::SubtractSmooth);
            ctx.constants.push(size.max(MIN_SMOOTHING));
        }
        Node::Intersect { lhs, rhs } => {
            compile_node(graph, *lhs, ctx, path);
            compile_node(graph, *rhs, ctx, path);
            ctx.opcodes.push(Opcode::Intersect);
        }
        Node::IntersectSmooth { lhs, rhs, size } => {
            compile_node(graph, *lhs, ctx, path);
            compile_node(graph, *rhs, ctx, path);
            ctx.opcodes.push(Opcode::IntersectSmooth);
            ctx.constants.push(size.max(MIN_SMOOTHING));
        }

        Node::Translate { translation, child } => {
            ctx.opcodes.push(Opcode::PushTranslation);
            ctx.constant_push_vec3(-*translation);
            compile_node(graph, *child, ctx, path);
            ctx.opcodes.push(Opcode::PopTransform);
        }
        Node::Rotate { rotation, child } => {
            ctx.opcodes.push(Opcode::PushRotation);
            ctx.constant_push_vec4(rotation.conjugate());
            compile_node(graph, *child, ctx, path);
            ctx.opcodes.push(Opcode::PopTransform);
        }
        Node::Scale { scale, child } => {
            ctx.opcodes.push(Opcode::PushScale);
            ctx.constants.push(1.0 / *scale);

            compile_node(graph, *child, ctx, path);

            ctx.opcodes.push(Opcode::PopScale);
            ctx.constants.push(*scale);
        }
        Node::Graph { root, graph } => {
            compile_node(graph, *root, ctx, &mut Vec::new());
        }
    }

    path.pop();
}

#[must_use]
pub fn compile(graph: &Graph, root: NodeId) -> Program {
    let mut program = Program::default();
    compile_node(graph, root, &mut program, &mut Vec::new());
    program.opcodes.push(Opcode::End);

    program
}

pub fn decompile(program: &Program, constants: &[f32]) -> Result<(Graph, NodeId), Error> {
    let mut graph = Graph::default();
    let mut stack = vec![];
    let mut hit_end = false;
    let mut constants = ConstantReader::new(constants);

    enum Transform {
        Translation(Vec3),
        Rotation(Quat),
    }

    let mut transform_stack: Vec<Transform> = vec![];

    for opcode in &program.opcodes {
        match opcode {
            Opcode::Sphere => {
                let center = constants.read_vec3()?;
                let radius = constants.read_f32()?;
                stack.push(graph.sphere(center, radius));
            }
            Opcode::Union => {
                let rhs = stack.pop().ok_or(Error::BadStack)?;
                let lhs = stack.pop().ok_or(Error::BadStack)?;
                stack.push(graph.op_union(lhs, rhs));
            }
            Opcode::UnionSmooth => {
                let rhs = stack.pop().ok_or(Error::BadStack)?;
                let lhs = stack.pop().ok_or(Error::BadStack)?;
                let smooth_size = constants.read_f32()?;
                stack.push(graph.op_union_smooth(lhs, rhs, smooth_size));
            }
            Opcode::Intersect => {
                let rhs = stack.pop().ok_or(Error::BadStack)?;
                let lhs = stack.pop().ok_or(Error::BadStack)?;
                stack.push(graph.op_intersect(lhs, rhs));
            }
            Opcode::IntersectSmooth => {
                let rhs = stack.pop().ok_or(Error::BadStack)?;
                let lhs = stack.pop().ok_or(Error::BadStack)?;
                let smooth_size = constants.read_f32()?;
                stack.push(graph.op_intersect_smooth(lhs, rhs, smooth_size));
            }
            Opcode::Subtract => {
                let rhs = stack.pop().ok_or(Error::BadStack)?;
                let lhs = stack.pop().ok_or(Error::BadStack)?;
                stack.push(graph.op_subtract(lhs, rhs));
            }
            Opcode::SubtractSmooth => {
                let rhs = stack.pop().ok_or(Error::BadStack)?;
                let lhs = stack.pop().ok_or(Error::BadStack)?;
                let smooth_size = constants.read_f32()?;
                stack.push(graph.op_subtract_smooth(lhs, rhs, smooth_size));
            }
            Opcode::RoundedBox => {
                let half_size = constants.read_vec3()?;
                let rounding_radius = constants.read_f32()?;
                stack.push(graph.rounded_box(half_size, rounding_radius));
            }
            Opcode::RoundedCylinder => {
                let cylinder_radius = constants.read_f32()?;
                let half_height = constants.read_f32()?;
                let rounding_radius = constants.read_f32()?;
                stack.push(graph.rounded_cylinder(cylinder_radius, half_height, rounding_radius));
            }
            Opcode::Cone => {
                let radius = constants.read_f32()?;
                let height = constants.read_f32()?;
                stack.push(graph.cone(radius, height));
            }
            Opcode::TaperedCapsule => {
                let point0 = constants.read_vec3()?;
                let radius0 = constants.read_f32()?;
                let point1 = constants.read_vec3()?;
                let radius1 = constants.read_f32()?;
                stack.push(graph.tapered_capsule([point0, point1], [radius0, radius1]));
            }
            Opcode::BiconvexLens => {
                let lower_sagitta = constants.read_f32()?;
                let upper_sagitta = constants.read_f32()?;
                let chord = constants.read_f32()?;
                stack.push(graph.biconvex_lens(lower_sagitta, upper_sagitta, chord));
            }
            Opcode::Capsule => {
                let point0 = constants.read_vec3()?;
                let point1 = constants.read_vec3()?;
                let radius = constants.read_f32()?;
                stack.push(graph.capsule([point0, point1], radius));
            }
            Opcode::Torus => {
                let big_r = constants.read_f32()?;
                let small_r = constants.read_f32()?;
                stack.push(graph.torus(big_r, small_r));
            }
            Opcode::TorusSector => {
                let big_r = constants.read_f32()?;
                let small_r = constants.read_f32()?;
                let half_angle = constants.read_f32()?;
                stack.push(graph.torus_sector(big_r, small_r, half_angle));
            }
            Opcode::Plane => {
                let plane = constants.read_vec4()?;
                stack.push(graph.plane(plane));
            }
            Opcode::Material => {
                let child = stack.pop().ok_or(Error::BadStack)?;
                let material = Material::from(constants.read_vec3()?);
                stack.push(graph.op_material(child, material));
            }
            Opcode::End => {
                hit_end = true;
                break;
            }
            Opcode::PushScale => {
                constants.skip(1);
            }
            Opcode::PopScale => {
                let child = stack.pop().ok_or(Error::BadStack)?;
                let scale = constants.read_f32()?;
                stack.push(graph.op_scale(child, scale));
            }
            Opcode::PushTranslation => {
                let translation = constants.read_vec3()?;
                transform_stack.push(Transform::Translation(-translation));
            }
            Opcode::PushRotation => {
                let rotation = constants.read_quat()?;
                transform_stack.push(Transform::Rotation(rotation.conjugate()));
            }
            Opcode::PopTransform => {
                let child = stack.pop().ok_or(Error::BadStack)?;
                match transform_stack.pop().ok_or(Error::BadStack)? {
                    Transform::Translation(translation) => {
                        stack.push(graph.op_translate(child, translation));
                    }
                    Transform::Rotation(rotation) => {
                        stack.push(graph.op_rotate(child, rotation));
                    }
                }
            }
        }
    }

    if stack.len() != 1 {
        return Err(Error::BadStack);
    }
    if !transform_stack.is_empty() {
        return Err(Error::BadStack);
    }
    if !hit_end {
        return Err(Error::BadProgram("Missing End"));
    }
    if !constants.at_end() {
        return Err(Error::BadProgram("Unused constants"));
    }
    Ok((graph, stack.pop().unwrap()))
}

pub fn disassemble(opcodes: &[Opcode], constants: &[f32]) -> Result<String, Error> {
    use std::fmt::Write;

    let mut s = String::with_capacity(opcodes.len() * 100);
    let mut constants = ConstantReader::new(constants);

    // TODO: Show the constants, too.
    for opcode in opcodes {
        match opcode {
            Opcode::Sphere => {
                let center = constants.read_vec3()?;
                let radius = constants.read_f32()?;
                let _ = writeln!(&mut s, "Sphere {} {}", center, radius);
            }
            Opcode::Union => {
                s.push_str("Union\n");
            }
            Opcode::UnionSmooth => {
                let smooth_size = constants.read_f32()?;
                let _ = writeln!(&mut s, "UnionSmooth {}", smooth_size);
            }
            Opcode::Intersect => {
                s.push_str("Intersect\n");
            }
            Opcode::IntersectSmooth => {
                let smooth_size = constants.read_f32()?;
                let _ = writeln!(&mut s, "IntersectSmooth {}", smooth_size);
            }
            Opcode::Subtract => {
                s.push_str("Subtract\n");
            }
            Opcode::SubtractSmooth => {
                let smooth_size = constants.read_f32()?;
                let _ = writeln!(&mut s, "SubtractSmooth {}", smooth_size);
            }
            Opcode::RoundedBox => {
                let half_size = constants.read_vec3()?;
                let rounding_radius = constants.read_f32()?;
                let _ = writeln!(&mut s, "RoundedBox {} {}", half_size, rounding_radius);
            }
            Opcode::RoundedCylinder => {
                let cylinder_radius = constants.read_f32()?;
                let half_height = constants.read_f32()?;
                let rounding_radius = constants.read_f32()?;
                let _ = writeln!(
                    &mut s,
                    "RoundedCylinder r={} h={} round={}",
                    cylinder_radius, half_height, rounding_radius
                );
            }
            Opcode::Cone => {
                let radius = constants.read_f32()?;
                let height = constants.read_f32()?;
                let _ = writeln!(&mut s, "Cone r={} h={}", radius, height);
            }
            Opcode::TaperedCapsule => {
                let point0 = constants.read_vec3()?;
                let radius0 = constants.read_f32()?;
                let point1 = constants.read_vec3()?;
                let radius1 = constants.read_f32()?;
                let _ = writeln!(
                    &mut s,
                    "TaperedCapsule p0={} r0={} p1={} r1={}",
                    point0, radius0, point1, radius1
                );
            }
            Opcode::BiconvexLens => {
                let lower_sagitta = constants.read_f32()?;
                let upper_sagitta = constants.read_f32()?;
                let chord = constants.read_f32()?;
                let _ = writeln!(
                    &mut s,
                    "BiconvexLens l={} u={} chord={}",
                    lower_sagitta, upper_sagitta, chord
                );
            }
            Opcode::Capsule => {
                let point0 = constants.read_vec3()?;
                let point1 = constants.read_vec3()?;
                let radius = constants.read_f32()?;
                let _ = writeln!(&mut s, "Capsule p0={} p1={} r={}", point0, point1, radius);
            }
            Opcode::Torus => {
                let big_r = constants.read_f32()?;
                let small_r = constants.read_f32()?;
                let _ = writeln!(&mut s, "Torus big_r={} small_R={}", big_r, small_r);
            }
            Opcode::TorusSector => {
                let big_r = constants.read_f32()?;
                let small_r = constants.read_f32()?;
                let half_angle = constants.read_f32()?;
                let _ = writeln!(
                    &mut s,
                    "TorusSector big_r={} small_R={} half_angle={}",
                    big_r, small_r, half_angle
                );
            }
            Opcode::Plane => {
                let plane = constants.read_vec4()?;
                let _ = writeln!(&mut s, "Plane: {}", plane);
            }
            Opcode::Material => {
                let material = constants.read_vec3()?;
                let _ = writeln!(&mut s, "Material: {}", material);
            }
            Opcode::End => {
                s.push_str("End\n");
                break;
            }
            Opcode::PushScale => {
                let scale = constants.read_f32()?;
                let _ = writeln!(&mut s, "PushScale: {}", scale);
            }
            Opcode::PopScale => {
                s.push_str("PopScale\n");
            }
            Opcode::PushTranslation => {
                let translation = constants.read_vec3()?;
                let _ = writeln!(&mut s, "PushTranslation: {}", translation);
            }
            Opcode::PushRotation => {
                let rotation = constants.read_quat()?;
                let _ = writeln!(&mut s, "PushRotation: {}", rotation);
            }
            Opcode::PopTransform => {
                s.push_str("PopTransform\n");
            }
        }
    }

    s.shrink_to_fit();
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut graph = Graph::default();
        let sphere1 = graph.sphere(Vec3::default(), 1.0);
        let rot_sphere1 = graph.op_rotate(sphere1, Quat::from_rotation_y(1.0));
        let sphere2 = graph.sphere(Vec3::default(), 5.0);
        let trans_sphere2 = graph.op_translate(sphere2, Vec3::ONE);
        let box2 = graph.rounded_box(Vec3::ONE, 0.1);
        let union = graph.op_union(rot_sphere1, trans_sphere2);
        let intersection = graph.op_intersect(union, box2);
        let scale = graph.op_scale(intersection, 1.5);

        let root = graph.op_rgb(scale, Vec3::new(1.0, 0.5, 0.0));

        let program = compile(&graph, root);

        // println!("comp:\n{}", program.disassemble());

        let (decomp_graph, decomp_root) = decompile(&program, &program.constants).unwrap();

        // We can't directly compare graphs easily, so we just compile again, then compare the output.
        let recomp_program = compile(&decomp_graph, decomp_root);

        // println!("recomp:\n{}", program.disassemble());

        // No longer doing the 1/x in scale, we use the non-inverted scale
        // from the "pop". So should match exactly.
        assert!(program.opcodes == recomp_program.opcodes);
        assert!(program.constants == recomp_program.constants);
    }
}
