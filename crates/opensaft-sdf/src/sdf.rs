#![allow(missing_docs, clippy::manual_clamp)]

use super::Material;
use super::SignedDistance;
use glam::*;
#[cfg(target_arch = "spirv")]
use num_traits::Float;

#[inline]
fn square_vec3(v: Vec3) -> f32 {
    v.dot(v)
}

#[inline]
fn hypot(v: Vec2) -> f32 {
    #[cfg(target_arch = "spirv")]
    let k = v.length();
    #[cfg(not(target_arch = "spirv"))]
    let k = v.x.hypot(v.y);
    k
}

#[inline]
pub fn sd_plane<T: SignedDistance>(pos: Vec3, plane: Vec4) -> T {
    T::new_with_distance(Material::default(), pos.dot(plane.truncate()) + plane.w)
}

#[inline]
pub fn sd_sphere<T: SignedDistance>(pos: Vec3, center: Vec3, radius: f32) -> T {
    T::new_with_distance(Material::default(), (pos - center).length() - radius)
}

#[inline]
pub fn sd_rounded_box<T: SignedDistance>(pos: Vec3, half_size: Vec3, rounding_radius: f32) -> T {
    let q = pos.abs() - half_size + Vec3::splat(rounding_radius);
    let dist = q.max(Vec3::splat(0.0)).length() + q.x.max(q.y.max(q.z)).min(0.0) - rounding_radius;
    T::new_with_distance(Material::default(), dist)
}

#[inline]
pub fn sd_torus<T: SignedDistance>(pos: Vec3, big_r: f32, small_r: f32) -> T {
    let q = Vec2::new(hypot(pos.xz()) - big_r, pos.y);
    let dist = q.length() - small_r;
    T::new_with_distance(Material::default(), dist)
}

#[inline]
pub fn sd_torus_sector<T: SignedDistance>(
    mut pos: Vec3,
    big_r: f32,
    small_r: f32,
    sin_cos_half_angle: (f32, f32),
) -> T {
    pos.x = pos.x.abs();
    let k = if sin_cos_half_angle.1 * pos.x > sin_cos_half_angle.0 * pos.z {
        pos.x * sin_cos_half_angle.0 + pos.z * sin_cos_half_angle.1
    } else {
        hypot(pos.xz())
    };
    let dist = (pos.dot(pos) + big_r.powi(2) - 2.0 * big_r * k)
        .max(0.0)
        .sqrt()
        - small_r;
    T::new_with_distance(Material::default(), dist)
}

#[inline]
pub fn sd_biconvex_lens<T: SignedDistance>(
    pos: Vec3,
    lower_sagitta: f32,
    upper_sagitta: f32,
    chord: f32,
) -> T {
    let chord_radius = chord / 2.0;
    let lower_radius = (chord_radius.powi(2) + lower_sagitta.powi(2)) / (2.0 * lower_sagitta);
    let upper_radius = (chord_radius.powi(2) + upper_sagitta.powi(2)) / (2.0 * upper_sagitta);
    let lower_center = Vec3::new(0.0, lower_radius - lower_sagitta, 0.0);
    let upper_center = Vec3::new(0.0, -(upper_radius - upper_sagitta), 0.0);
    sd_op_intersect(
        sd_sphere(pos, lower_center, lower_radius),
        sd_sphere(pos, upper_center, upper_radius),
    )
}

#[inline]
pub fn sd_capsule<T: SignedDistance>(pos: Vec3, points: &[Vec3; 2], radius: f32) -> T {
    let pa = pos - points[0];
    let ba = points[1] - points[0];
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    let distance = (pa - ba * h).length() - radius;
    T::new_with_distance(Material::default(), distance)
}

#[inline]
pub fn sd_rounded_cylinder_f(
    pos: Vec3,
    cylinder_radius: f32,
    half_height: f32,
    rounding_radius: f32,
) -> f32 {
    let d = Vec2::new(
        hypot(pos.xz()) - cylinder_radius + rounding_radius,
        pos.y.abs() - half_height + rounding_radius,
    );
    d.x.max(d.y).min(0.0) + d.max(Vec2::ZERO).length() - rounding_radius
}

#[inline]
pub fn sd_rounded_cylinder<T: SignedDistance>(
    pos: Vec3,
    cylinder_radius: f32,
    half_height: f32,
    rounding_radius: f32,
) -> T {
    T::new_with_distance(
        Material::default(),
        sd_rounded_cylinder_f(pos, cylinder_radius, half_height, rounding_radius),
    )
}

#[allow(clippy::many_single_char_names)]
#[inline]
pub fn sd_tapered_capsule_f(pos: Vec3, p: &[Vec3; 2], r: [f32; 2]) -> f32 {
    // Straight from https://www.iquilezles.org/www/articles/distfunctions/distfunctions.htm

    // sampling independent computations (only depend on shape)
    let ba = p[1] - p[0];
    let l2 = ba.dot(ba);
    let rr = r[0] - r[1];
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;

    // sampling dependant computations
    let pa = pos - p[0];
    let y = pa.dot(ba);
    let z = y - l2;
    let x2 = square_vec3(pa * l2 - ba * y);
    let y2 = y * y * l2;
    let z2 = z * z * l2;

    // single square root!
    let k = rr.signum() * rr * rr * x2;
    if z.signum() * a2 * z2 > k {
        (x2 + z2).sqrt() * il2 - r[1]
    } else if y.signum() * a2 * y2 < k {
        (x2 + y2).sqrt() * il2 - r[0]
    } else {
        (y * rr + (x2 * a2 * il2).sqrt()) * il2 - r[0]
    }
}

#[inline]
pub fn sd_tapered_capsule<T: SignedDistance>(pos: Vec3, p: &[Vec3; 2], r: [f32; 2]) -> T {
    T::new_with_distance(Material::default(), sd_tapered_capsule_f(pos, p, r))
}

/// Base at origin, with height `h` along positive Y.
#[allow(clippy::many_single_char_names)]
#[inline]
pub fn sd_cone_f(p: Vec3, r: f32, h: f32) -> f32 {
    let q = vec2(r, h);
    let w = vec2(hypot(p.xz()), h - p.y);
    let a = w - q * (w.dot(q) / q.dot(q)).clamp(0.0, 1.0);
    let b = w - vec2(r * (w.x / r).clamp(0.0, 1.0), h);
    let d = a.dot(a).min(b.dot(b));
    let s = (w.x * h - w.y * r).max(w.y - h);
    d.sqrt() * s.signum()
}

#[inline]
pub fn sd_cone<T: SignedDistance>(pos: Vec3, r: f32, h: f32) -> T {
    T::new_with_distance(Material::default(), sd_cone_f(pos, r, h))
}

#[inline]
pub fn sd_material<T: SignedDistance>(sd: T, material: Material) -> T {
    T::new_with_distance(material, sd.distance())
}

#[inline]
pub fn sd_op_union<T: SignedDistance>(d1: T, d2: T) -> T {
    if d1.distance() < d2.distance() {
        d1
    } else {
        d2
    }
}

#[inline]
pub fn sd_op_subtract<T: SignedDistance>(d1: T, d2: T) -> T {
    let neg_distance1 = -d1.distance();
    let distance2 = d2.distance();
    if neg_distance1 > distance2 {
        d1.copy_with_distance(neg_distance1)
    } else {
        d2
    }
}

#[inline]
pub fn sd_op_intersect<T: SignedDistance>(d1: T, d2: T) -> T {
    if d1.distance() > d2.distance() {
        d1
    } else {
        d2
    }
}

#[inline]
pub fn sd_op_union_smooth<T: SignedDistance>(d1: T, d2: T, size: f32) -> T {
    let h = 0.5 + 0.5 * (d2.distance() - d1.distance()) / size;
    let h = h.clamp(0.0, 1.0);

    let new_d = d2.lerp(&d1, h);

    let distance = new_d.distance() - size * h * (1.0 - h);

    new_d.copy_with_distance(distance)
}

#[inline]
pub fn sd_op_subtract_smooth<T: SignedDistance>(d1: T, d2: T, size: f32) -> T {
    let h = 0.5 - 0.5 * (d2.distance() + d1.distance()) / size;
    let h = h.clamp(0.0, 1.0);

    let d1 = d1.copy_with_distance(-d1.distance());

    let new_d = d2.lerp(&d1, h);

    let distance = (size * h) * (1.0 - h) + new_d.distance();

    new_d.copy_with_distance(distance)
}

#[inline]
pub fn sd_op_intersect_smooth<T: SignedDistance>(d1: T, d2: T, size: f32) -> T {
    let h = 0.5 - 0.5 * (d2.distance() - d1.distance()) / size;
    let h = h.clamp(0.0, 1.0);

    let new_d = d2.lerp(&d1, h);

    let distance = (size * h) * (1.0 - h) + new_d.distance();

    new_d.copy_with_distance(distance)
}
