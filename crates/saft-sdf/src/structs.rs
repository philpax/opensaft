use macaw::Vec3;
use macaw::Vec4;

#[derive(Copy, Clone)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(not(target_arch = "spirv"), derive(Debug))]
pub struct Material {
    /// [0-1] linear space
    rgb: Vec3,
}

impl Default for Material {
    fn default() -> Self {
        Self::new(Vec3::ONE)
    }
}

impl From<Vec3> for Material {
    fn from(rgb: Vec3) -> Self {
        Self { rgb }
    }
}

impl Material {
    pub fn new(rgb: Vec3) -> Self {
        Self { rgb }
    }

    pub fn rgb(&self) -> Vec3 {
        self.rgb
    }
}

pub trait SignedDistance: Copy {
    #[must_use]
    fn infinity() -> Self;

    fn distance(&self) -> f32;

    #[must_use]
    fn copy_with_distance(&self, distance: f32) -> Self;

    #[must_use]
    fn multiply_distance_by(&self, factor: f32) -> Self;

    fn material(&self) -> Material;

    #[must_use]
    fn lerp(&self, b: &Self, t: f32) -> Self;

    #[must_use]
    fn new_with_distance(material: Material, distance: f32) -> Self;

    fn is_distance_finite(&self) -> bool;
}

impl SignedDistance for f32 {
    #[inline]
    fn infinity() -> Self {
        Self::INFINITY
    }

    #[inline]
    fn distance(&self) -> f32 {
        *self
    }

    #[inline]
    fn material(&self) -> Material {
        Material::new(Vec3::ONE)
    }

    #[inline]
    fn copy_with_distance(&self, distance: f32) -> Self {
        distance
    }

    #[inline]
    fn multiply_distance_by(&self, factor: f32) -> Self {
        self * factor
    }

    #[inline]
    fn new_with_distance(_material: Material, distance: f32) -> Self {
        distance
    }

    #[inline]
    fn lerp(&self, b: &Self, t: f32) -> Self {
        //Self(self + (b.0 - self) * t)
        t * (b - self) + *self
    }

    #[inline]
    fn is_distance_finite(&self) -> bool {
        self.is_finite()
    }
}

/// r, g, b, distance
#[derive(Default, Copy, Clone, PartialEq)]
#[cfg_attr(not(target_arch = "spirv"), derive(Debug))]
pub struct RgbWithDistance(pub Vec4);

impl SignedDistance for RgbWithDistance {
    #[inline]
    fn infinity() -> Self {
        Self(Vec4::new(1.0, 1.0, 1.0, core::f32::INFINITY))
    }

    #[inline]
    fn distance(&self) -> f32 {
        self.0.w
    }

    #[inline]
    fn material(&self) -> Material {
        Material::new(self.0.truncate())
    }

    #[inline]
    fn copy_with_distance(&self, distance: f32) -> Self {
        let mut n = *self;
        n.0.w = distance;
        n
    }

    #[inline]
    fn multiply_distance_by(&self, factor: f32) -> Self {
        Self(self.0.truncate().extend(self.0.w * factor))
    }

    #[inline]
    fn new_with_distance(material: Material, distance: f32) -> Self {
        Self(material.rgb().extend(distance))
    }

    #[inline]
    fn lerp(&self, b: &Self, t: f32) -> Self {
        Self(self.0.lerp(b.0, t))
    }

    #[inline]
    fn is_distance_finite(&self) -> bool {
        self.0.is_finite()
    }
}
