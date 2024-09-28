use macaw::Vec3;

use crate::SignedDistance;

// TODO: use u32 as index? Should be large enough.
// Or replace with IVec3?
pub type Index3 = [usize; 3];

/// Stores values on a 3D cube lattice on the coordinates \[0,0,0\] - \[w-1, h-1, d-1\].
/// A 3D tensor, basically.
///
/// TODO (nummelin): Should we separate storage so we can store just a bitset of inside/outside (which is enough for most uses).
pub struct Grid3<T = f32> {
    size: Index3,
    data: Vec<T>,
}

impl<T> Grid3<T> {
    /// flat data
    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn size(&self) -> Index3 {
        self.size
    }
}

impl<T> std::ops::Index<Index3> for Grid3<T> {
    type Output = T;

    #[inline]
    fn index(&self, p: Index3) -> &Self::Output {
        debug_assert!(p[0] < self.size[0]);
        debug_assert!(p[1] < self.size[1]);
        debug_assert!(p[2] < self.size[2]);
        &self.data[p[0] + self.size[0] * (p[1] + self.size[1] * p[2])]
    }
}

impl<T> std::ops::IndexMut<Index3> for Grid3<T> {
    #[inline]
    fn index_mut(&mut self, p: Index3) -> &mut Self::Output {
        debug_assert!(p[0] < self.size[0]);
        debug_assert!(p[1] < self.size[1]);
        debug_assert!(p[2] < self.size[2]);
        &mut self.data[p[0] + self.size[0] * (p[1] + self.size[1] * p[2])]
    }
}

impl<T: std::cmp::PartialEq> std::cmp::PartialEq for Grid3<T> {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size && self.data == other.data
    }
}

impl<T: SignedDistance + Copy + Clone + Default> Grid3<T> {
    pub fn new(size: Index3) -> Self {
        Self {
            size,
            data: vec![T::default(); size[0] * size[1] * size[2]],
        }
    }

    /// Set the grid values using the given function.
    pub fn set(&mut self, mut f: impl FnMut(Index3) -> T) {
        let mut index = 0;
        for z in 0..self.size[2] {
            for y in 0..self.size[1] {
                for x in 0..self.size[0] {
                    self.data[index] = f([x, y, z]);
                    index += 1;
                }
            }
        }
    }
}

impl<T> Grid3<T>
where
    T: SignedDistance,
{
    /// Returns the distance gradient at the given coordinate.
    /// Coordinate must be within the grid.
    #[inline]
    pub fn gradient_clamped(&self, p: Index3) -> Vec3 {
        let p = [
            p[0].clamp(1, self.size[0] - 2),
            p[1].clamp(1, self.size[1] - 2),
            p[2].clamp(1, self.size[2] - 2),
        ];
        let dx = self[[p[0] + 1, p[1], p[2]]].distance() - self[[p[0] - 1, p[1], p[2]]].distance();
        let dy = self[[p[0], p[1] + 1, p[2]]].distance() - self[[p[0], p[1] - 1, p[2]]].distance();
        let dz = self[[p[0], p[1], p[2] + 1]].distance() - self[[p[0], p[1], p[2] - 1]].distance();
        Vec3::new(dx, dy, dz) / 2.0
    }

    /// Returns a fast approximation of the distance gradient at the given coordinate.
    ///
    /// Coordinate must be within the grid.
    #[inline]
    pub fn fast_gradient(
        &self,
        x: usize,
        y: usize,
        z: usize,
        i: usize,  // index of x, y, z
        ys: usize, // y stride
        zs: usize, // z stride
    ) -> Vec3 {
        let sx = self.size[0];
        let sy = self.size[1];
        let sz = self.size[2];

        let x1 = if x < sx - 1 { i + 1 } else { i };
        let x2 = if x > 0 { i - 1 } else { i };
        let y1 = if y < sy - 1 { i + ys } else { i };
        let y2 = if y > 0 { i - ys } else { i };
        let z1 = if z < sz - 1 { i + zs } else { i };
        let z2 = if z > 0 { i - zs } else { i };

        let dx = self.data[x1].distance() - self.data[x2].distance();
        let dy = self.data[y1].distance() - self.data[y2].distance();
        let dz = self.data[z1].distance() - self.data[z2].distance();

        Vec3::new(dx, dy, dz) // (should divide by 2 here, but it doesn't matter as we normalize later)
    }

    fn set_truncated_span(
        x_slice: &mut [T],
        y: usize,
        z: usize,
        sdf: impl Fn(Index3) -> T + Send + Sync,
        truncate_dist: f32,
    ) {
        let w = x_slice.len();
        let mut x = 0;

        while x < w {
            let distance = sdf([x, y, z]);
            let abs_distance = distance.distance().abs();

            x_slice[x] = distance;
            x += 1;

            let mut distance_bound = abs_distance - 1.0;
            while distance_bound > truncate_dist && x < w {
                x_slice[x] = distance;
                x += 1;

                distance_bound -= 1.0;
            }
        }
    }

    /// Will set all values closer than the given truncate distance
    ///
    /// Cells outside the given truncate distance will have approximated distances.
    ///
    /// Will run synchronously regardless of `with_rayon` feature availability.
    pub fn set_truncated_sync(
        &mut self,
        sdf: impl Fn(Index3) -> T + Send + Sync,
        truncate_dist: f32,
    ) {
        let _d = self.size[2];
        let h = self.size[1];
        let w = self.size[0];

        self.data
            .chunks_mut(w * h)
            .enumerate()
            .for_each(|(z, xy_plane)| {
                xy_plane.chunks_mut(w).enumerate().for_each(|(y, x_slice)| {
                    Self::set_truncated_span(x_slice, y, z, &sdf, truncate_dist);
                });
            });
    }

    /// Will set all values closer than the given truncate distance
    ///
    /// Cells outside the given truncate distance will have approximated distances.
    #[cfg(not(feature = "with_rayon"))]
    pub fn set_truncated(&mut self, sdf: impl Fn(Index3) -> T + Send + Sync, truncate_dist: f32) {
        self.set_truncated_sync(sdf, truncate_dist);
    }

    /// Will set all values closer than the given truncate distance
    ///
    /// Cells outside the given truncate distance will have approximated distances.
    #[cfg(feature = "with_rayon")]
    pub fn set_truncated(&mut self, sdf: impl Fn(Index3) -> T + Send + Sync, truncate_dist: f32)
    where
        T: Send,
    {
        let _d = self.size[2];
        let h = self.size[1];
        let w = self.size[0];

        use rayon::prelude::*;

        self.data
            .par_chunks_mut(w * h)
            .enumerate()
            .for_each(|(z, xy_plane)| {
                xy_plane
                    .par_chunks_mut(w)
                    .enumerate()
                    .for_each(|(y, x_slice)| {
                        Self::set_truncated_span(x_slice, y, z, &sdf, truncate_dist);
                    });
            });
    }
}

// TODO: Optimize updating and meshing by
// evaluating the sdf recursively / divide and conquer
// Start with big blocks, test sdf. If distance is bigger than block
// skip it and produce a bitset that can be used when doing the iso surface
// extraction.
