/// Assembly. Lower level representation.
#[derive(Copy, Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "with_opcode_derives",
    derive(Debug, Hash, num_enum::IntoPrimitive, num_enum::TryFromPrimitive)
)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "with_arbitrary", derive(arbitrary::Arbitrary))]
#[repr(u32)]
pub enum Opcode {
    // Primitives:
    Plane = 0,          // vec4
    Sphere = 1,         // center: vec3, radius: f32
    Capsule = 2,        // p0: vec3, p1: vec3, radius: f32
    TaperedCapsule = 3, // p0: vec3, r0: f32, p1: vec3, r0: f32

    Material = 4, // rgb: vec3

    // Combinators:
    Union = 5,
    UnionSmooth = 6,
    Subtract = 7,
    SubtractSmooth = 8,
    Intersect = 9,
    IntersectSmooth = 10,

    // Transforms:
    PushTranslation = 11,
    PushRotation = 12,
    PopTransform = 13,
    PushScale = 14,
    PopScale = 15,

    End = 16,

    RoundedBox = 17,      // half_size: vec3, radius: f32
    BiconvexLens = 18,    // lower_sagitta, upper_sagitta, chord
    RoundedCylinder = 19, // cylinder_radius, half_height, rounding_radius
    Torus = 20,           // big_r, small_r
    TorusSector = 21,     // big_r, small_r, sin_half_angle, cos_half_angle
    Cone = 22,            // radius, height
}
