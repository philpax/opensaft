//use super::Material;
use super::Opcode;
//use super::SignedDistance;
use crate::sdf;
use crate::structs::Material;
use crate::structs::SignedDistance;
use macaw::Quat;
use macaw::Vec3;
use macaw::Vec4;

#[derive(Copy, Clone)]
pub struct Interpreter<SD: SignedDistance, const STACK_DEPTH: usize = 64> {
    marker: core::marker::PhantomData<SD>,
}

impl<SD: SignedDistance> Default for Interpreter<SD> {
    fn default() -> Self {
        Self {
            marker: Default::default(),
        }
    }
}

pub struct InterpreterContext<'a, SD: SignedDistance, const STACK_DEPTH: usize = 64> {
    opcodes: &'a [Opcode],
    constants: &'a [f32],

    stack: [SD; STACK_DEPTH],
    stack_ptr: usize,
    constant_idx: usize,
    position_stack: [Vec3; STACK_DEPTH],
    position_stack_ptr: usize,
}

fn uninit<T>(_t: T) -> T {
    #[cfg(not(target_arch = "spirv"))]
    // SAFETY: We're using this for a stack where we don't care about initialized state.
    let ret = unsafe { core::mem::MaybeUninit::<T>::zeroed().assume_init() };

    #[cfg(target_arch = "spirv")]
    let ret = _t; // https://github.com/EmbarkStudios/rust-gpu/issues/981

    ret
}

impl<'a, SD: SignedDistance + Copy + Clone, const STACK_DEPTH: usize>
    InterpreterContext<'a, SD, STACK_DEPTH>
{
    fn new(opcodes: &'a [Opcode], constants: &'a [f32]) -> Self {
        Self {
            opcodes,
            constants,
            stack: uninit([SignedDistance::infinity(); STACK_DEPTH]),
            stack_ptr: 0,
            constant_idx: 0,
            position_stack: uninit([Vec3::ZERO; STACK_DEPTH]),
            position_stack_ptr: 0,
        }
    }

    fn reset(&mut self) {
        self.stack_ptr = 0;
        self.position_stack_ptr = 0;
        self.constant_idx = 0;
    }

    fn float32(&mut self) -> f32 {
        let ret = self.constants[self.constant_idx];
        self.constant_idx += 1;
        ret
    }

    fn vec3(&mut self) -> Vec3 {
        Vec3::new(self.float32(), self.float32(), self.float32())
    }

    // TODO (nummelin): How much faster is it to only allow 16-bytes alignment
    // when running through spirv?
    fn vec4(&mut self) -> Vec4 {
        Vec4::new(
            self.float32(),
            self.float32(),
            self.float32(),
            self.float32(),
        )
    }

    fn quat(&mut self) -> Quat {
        Quat::from_xyzw(
            self.float32(),
            self.float32(),
            self.float32(),
            self.float32(),
        )
    }

    fn material(&mut self) -> Material {
        self.vec3().into()
    }

    fn push_sd(&mut self, v: SD) {
        self.stack[self.stack_ptr] = v;
        self.stack_ptr += 1;
    }

    fn pop_sd(&mut self) -> Option<SD> {
        self.stack_ptr -= 1;
        self.stack.get(self.stack_ptr).copied()
    }

    fn pop_sd_unchecked(&mut self) -> SD {
        self.stack_ptr -= 1;
        self.stack[self.stack_ptr]
    }

    fn push_position(&mut self, pos: Vec3) {
        self.position_stack[self.position_stack_ptr] = pos;
        self.position_stack_ptr += 1;
    }

    fn pop_position_unchecked(&mut self) -> Vec3 {
        self.position_stack_ptr -= 1;
        self.position_stack[self.position_stack_ptr]
    }

    // See comment at the end of the `interpret` function below.
    #[allow(dead_code)]
    fn top_is_finite(&self) -> bool {
        if self.stack_ptr > 0 {
            if let Some(value) = self.stack.get(self.stack_ptr - 1) {
                value.is_distance_finite()
            } else {
                // Wacky
                false
            }
        } else {
            false
        }
    }
}

impl<SD: SignedDistance + Copy + Clone, const STACK_DEPTH: usize> Interpreter<SD, STACK_DEPTH> {
    pub fn new_context<'a>(
        opcodes: &'a [Opcode],
        constants: &'a [f32],
    ) -> InterpreterContext<'a, SD, STACK_DEPTH> {
        InterpreterContext::<SD, STACK_DEPTH>::new(opcodes, constants)
    }
    pub fn interpret(
        ctx: &mut InterpreterContext<'_, SD, STACK_DEPTH>,
        position: Vec3,
    ) -> Option<SD> {
        Self::interpret_internal(ctx, position);
        ctx.pop_sd()
    }

    pub fn interpret_unchecked(
        ctx: &mut InterpreterContext<'_, SD, STACK_DEPTH>,
        position: Vec3,
    ) -> SD {
        Self::interpret_internal(ctx, position);
        ctx.pop_sd_unchecked()
    }

    fn interpret_internal(ctx: &mut InterpreterContext<'_, SD, STACK_DEPTH>, position: Vec3) {
        #[allow(clippy::enum_glob_use)]
        use Opcode::*;

        let mut current_position = position;

        ctx.reset();

        let mut pc = 0;

        loop {
            let opcode = ctx.opcodes[pc];
            pc += 1;

            match opcode {
                Plane => {
                    let sd = sdf::sd_plane(current_position, ctx.vec4());
                    ctx.push_sd(sd);
                }
                Sphere => {
                    let sd = sdf::sd_sphere(current_position, ctx.vec3(), ctx.float32());
                    ctx.push_sd(sd);
                }
                Capsule => {
                    let sd =
                        sdf::sd_capsule(current_position, &[ctx.vec3(), ctx.vec3()], ctx.float32());
                    ctx.push_sd(sd);
                }
                RoundedCylinder => {
                    let sd = sdf::sd_rounded_cylinder(
                        current_position,
                        ctx.float32(),
                        ctx.float32(),
                        ctx.float32(),
                    );
                    ctx.push_sd(sd);
                }
                TaperedCapsule => {
                    let p0 = ctx.vec3();
                    let r0 = ctx.float32();
                    let p1 = ctx.vec3();
                    let r1 = ctx.float32();
                    let sd = sdf::sd_tapered_capsule(current_position, &[p0, p1], [r0, r1]);
                    ctx.push_sd(sd);
                }
                Cone => {
                    let r = ctx.float32();
                    let h = ctx.float32();
                    let sd = sdf::sd_cone(current_position, r, h);
                    ctx.push_sd(sd);
                }
                RoundedBox => {
                    let half_size = ctx.vec3();
                    let radius = ctx.float32();
                    let sd = sdf::sd_rounded_box(current_position, half_size, radius);
                    ctx.push_sd(sd);
                }
                Torus => {
                    let big_r = ctx.float32();
                    let small_r = ctx.float32();
                    ctx.push_sd(sdf::sd_torus(current_position, big_r, small_r));
                }
                TorusSector => {
                    let big_r = ctx.float32();
                    let small_r = ctx.float32();
                    let sin_cos_half_angle = (ctx.float32(), ctx.float32());
                    ctx.push_sd(sdf::sd_torus_sector(
                        current_position,
                        big_r,
                        small_r,
                        sin_cos_half_angle,
                    ));
                }
                BiconvexLens => {
                    let lower_sagitta = ctx.float32();
                    let upper_sagitta = ctx.float32();
                    let chord = ctx.float32();
                    let sd = sdf::sd_biconvex_lens(
                        current_position,
                        lower_sagitta,
                        upper_sagitta,
                        chord,
                    );
                    ctx.push_sd(sd);
                }
                Material => {
                    let sd = ctx.pop_sd_unchecked();
                    let material = ctx.material();
                    ctx.push_sd(sdf::sd_material(sd, material));
                }
                Union => {
                    let sd1 = ctx.pop_sd_unchecked();
                    let sd2 = ctx.pop_sd_unchecked();
                    ctx.push_sd(sdf::sd_op_union(sd1, sd2));
                }
                UnionSmooth => {
                    let sd1 = ctx.pop_sd_unchecked();
                    let sd2 = ctx.pop_sd_unchecked();
                    let width = ctx.float32();
                    ctx.push_sd(sdf::sd_op_union_smooth(sd1, sd2, width));
                }
                Subtract => {
                    let sd1 = ctx.pop_sd_unchecked();
                    let sd2 = ctx.pop_sd_unchecked();
                    ctx.push_sd(sdf::sd_op_subtract(sd1, sd2));
                }
                SubtractSmooth => {
                    let sd1 = ctx.pop_sd_unchecked();
                    let sd2 = ctx.pop_sd_unchecked();
                    let width = ctx.float32();
                    ctx.push_sd(sdf::sd_op_subtract_smooth(sd1, sd2, width));
                }
                Intersect => {
                    let sd1 = ctx.pop_sd_unchecked();
                    let sd2 = ctx.pop_sd_unchecked();
                    ctx.push_sd(sdf::sd_op_intersect(sd1, sd2));
                }
                IntersectSmooth => {
                    let sd1 = ctx.pop_sd_unchecked();
                    let sd2 = ctx.pop_sd_unchecked();
                    let width = ctx.float32();
                    ctx.push_sd(sdf::sd_op_intersect_smooth(sd1, sd2, width));
                }
                PushTranslation => {
                    let translation = ctx.vec3();
                    ctx.push_position(current_position);
                    current_position += translation;
                }
                PopTransform => {
                    current_position = ctx.pop_position_unchecked();
                }
                PushRotation => {
                    let rotation = ctx.quat();
                    ctx.push_position(current_position);
                    current_position = rotation * current_position;
                }
                PushScale => {
                    let inv_scale = ctx.float32();
                    ctx.push_position(current_position);
                    current_position *= inv_scale;
                }
                PopScale => {
                    current_position = ctx.pop_position_unchecked();
                    let scale = ctx.float32();
                    let sd = ctx.pop_sd_unchecked();
                    ctx.push_sd(sd.copy_with_distance(scale * sd.distance()));
                }
                End => {
                    break;
                }
            }

            // NaN check for debugging! Don't want the overhead by default, so disabled.
            // if !ctx.top_is_finite() {
            //    panic!("Hit infinity at {:?}", opcode);
            // }
        }
    }
}
