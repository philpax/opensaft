use macaw::Ray3;
use macaw::Vec3;
use std::cmp::Ordering;
use std::ops::RangeInclusive;

pub struct Options {
    /// Don't take more steps than this
    max_steps: usize,

    /// 1.0. Set to lower if your field is unreliable (i.e. underestimates distances).
    step_constant: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_steps: 1024,
            step_constant: 1.0,
        }
    }
}

/// A point along the ray, and some info about it.
/// Often this is the point along a march that was (approximately) closest to a surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClosestHit {
    /// Distance along ray.
    pub t: f32,
    /// Point in world.
    pub pos: Vec3,
    /// Distance to surface.
    pub dist: f32,
    /// Is this point considered a hit on the surface?
    /// If false, this point is the closest point we've found to a surface.
    pub is_hit: bool,
}

impl Default for ClosestHit {
    fn default() -> Self {
        Self::miss()
    }
}

impl ClosestHit {
    pub fn miss() -> Self {
        Self {
            t: f32::INFINITY,
            pos: Vec3::splat(f32::NAN),
            dist: f32::INFINITY,
            is_hit: false,
        }
    }

    /// How close was the hit, as seen from the ray origin?
    pub fn angle_distance(&self) -> f32 {
        if self.t <= self.dist {
            f32::INFINITY // angle doesn't make sense
        } else {
            self.dist / self.t
        }
    }
}

/// Less means earlier or closer hit.
impl std::cmp::PartialOrd for ClosestHit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.is_hit, other.is_hit) {
            (true, false) => Some(Ordering::Less),    // hits before misses
            (false, true) => Some(Ordering::Greater), // misses after hits
            (true, true) => self.t.partial_cmp(&other.t), // both hits: first hit is first
            (false, false) => self.angle_distance().partial_cmp(&other.angle_distance()), // both missed: closest is first
        }
    }
}

/// Marches a ray from `t_range.start()` until `t_range.end()`,
/// returning the first hit or (in case of no hit) the point that were closest to the surface.
///
/// `sd` is a signed distance field that should never underestimate the distance to the surface.
/// The surface is where the signed distance is zero.
pub fn trace(
    mut sd: impl FnMut(Vec3) -> f32,
    ray: Ray3,
    t_range: RangeInclusive<f32>,
    opt: &Options,
) -> ClosestHit {
    let mut t = *t_range.start();
    let mut closest_angle_distance = f32::INFINITY;
    let mut closest = ClosestHit::miss();

    for _ in 0..opt.max_steps {
        let pos = ray.point_along(t);
        let dist = sd(pos);
        if dist <= 0.001 * t {
            return ClosestHit {
                t,
                pos,
                dist,
                is_hit: true,
            };
        } else {
            if t > 0.0 {
                // `angle_distance`: dist as viewed from ray origin
                let angle_distance = dist / t;
                if angle_distance < closest_angle_distance {
                    closest_angle_distance = angle_distance;
                    closest = ClosestHit {
                        t,
                        pos,
                        dist,
                        is_hit: false,
                    };
                }
            }

            t += dist * opt.step_constant;

            if t >= *t_range.end() {
                return closest;
            }
        }
    }

    // TODO: communicate that max_steps has been reached,
    // i.e. that the march was prematurely aborted
    closest
}
