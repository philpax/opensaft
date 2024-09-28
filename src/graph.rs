use super::Material;
use crate::math::BoundingBox;
use glam::Quat;
use glam::Vec3;
use glam::Vec4;
use std::collections::HashMap;
use std::hash::Hash;

/// A high-level definition of a signed distance field function
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
pub struct Graph {
    id_allocator: u32,
    nodes: HashMap<NodeId, Node>, // TODO (nummelin): This should really be a Vec?
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
pub struct NodeId(u32);

#[derive(Clone, Debug)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
pub enum Node {
    // Primitives:
    /// distance = plane.xyz.dot(pos) + plane.w.
    /// The plane normal (plane.xyz) should be unit length (normalized).
    Plane(Vec4),

    Sphere {
        center: Vec3,
        radius: f32,
    },

    /// The full length of the capsule is `(points[0] - points[1]).length() + 2.0 * radius`
    Capsule {
        /// The centers of the endpoints of the capsule
        points: [Vec3; 2],
        /// The radius of the capsule.
        radius: f32,
    },

    /// A cylinder with rounded edges.
    /// The cylinder has the center in origin, and stretches along the Y axis.
    ///
    /// The rounding is subtracted from the edges (sandpapered down).
    ///
    /// When `rounding_radius == 2 * cylinder_radius` you get a capsule.
    ///
    /// When `half_height = rounding_radius` you get a filled torus.
    RoundedCylinder {
        cylinder_radius: f32,
        half_height: f32,
        rounding_radius: f32,
    },

    /// The convex hull of two spheres.
    ///
    /// The full length of the round cone is `(points[0] - points[1]).length() + radii[0] + radii[1]`
    TaperedCapsule {
        points: [Vec3; 2],
        radii: [f32; 2],
    },

    /// Base center at origin, extending `height` along positive Y axis.
    Cone {
        /// Radius of the base
        radius: f32,
        /// Height of the cone
        height: f32,
    },

    /// A box with rounded edges / corners.
    ///
    /// The rounding is subtracted from the edges and corners (sandpapered down).
    RoundedBox {
        half_size: Vec3,
        rounding_radius: f32,
    },

    /// A ring, e.g. a donut.
    ///
    /// Centered at origin, lying in the XZ plane.
    Torus {
        big_r: f32,
        small_r: f32,
    },

    /// Part of a ring, e.g. a donut someone has taken a bite out of.
    ///
    /// Centered at origin, lying in the XZ plane.
    /// The missing piece of the torus is in the negative Z direction.
    TorusSector {
        big_r: f32,
        small_r: f32,
        /// The sin/cos of the half-angle, so that half_angle=PI means full torus, half_angle=PI/2 means half torus, etc.
        sin_cos_half_angle: (f32, f32),
    },

    /// Biconvex lens
    ///
    /// The surface is described by two spherical caps with the same base diameter (chord) and a lower
    /// and an upper sagitta defining the height of each spherical cap.
    ///
    ///  (seen from xy-plane)
    ///
    /// ```text
    ///      _.-"|"-._
    ///    .'    |    `.
    ///   /      |upper \
    ///  |chord  |       |
    ///  |---------------|
    ///  |       |       |
    ///   \      |lower /
    ///    `-..._|_...-'
    /// ```
    ///
    BiconvexLens {
        lower_sagitta: f32,
        upper_sagitta: f32,
        chord: f32,
    },

    /// Set material of all child nodes:
    Material {
        child: NodeId,
        material: Material,
    },

    // Combinations:
    Union {
        lhs: NodeId,
        rhs: NodeId,
    },
    UnionMulti {
        children: Vec<NodeId>,
    },
    UnionSmooth {
        lhs: NodeId,
        rhs: NodeId,
        size: f32,
    },
    UnionMultiSmooth {
        children: Vec<NodeId>,
        size: f32,
    },
    Subtract {
        lhs: NodeId,
        rhs: NodeId,
    },
    SubtractSmooth {
        lhs: NodeId,
        rhs: NodeId,
        size: f32,
    },
    Intersect {
        lhs: NodeId,
        rhs: NodeId,
    },
    IntersectSmooth {
        lhs: NodeId,
        rhs: NodeId,
        size: f32,
    },

    // Transforms:
    Translate {
        translation: Vec3,
        child: NodeId,
    },
    Rotate {
        rotation: Quat,
        child: NodeId,
    },
    Scale {
        scale: f32,
        child: NodeId,
    },
    // Yo dawg, I heard you like graphs:
    Graph {
        root: NodeId,
        graph: Graph,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CsgOp {
    Union,
    Subtract,
    Intersect,
}

// Constructors
impl Graph {
    pub fn create_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.id_allocator);
        self.id_allocator += 1;
        self.nodes.insert(id, node);
        id
    }

    pub fn graph(&mut self, graph: Self, root: NodeId) -> NodeId {
        let id = NodeId(self.id_allocator);
        self.id_allocator += 1;
        self.nodes.insert(id, Node::Graph { root, graph });
        id
    }

    /// distance = plane.xyz.dot(pos) + plane.w
    /// The plane normal (plane.xyz) should be unit length (normalized).
    pub fn plane(&mut self, plane: Vec4) -> NodeId {
        self.create_node(Node::Plane(plane))
    }

    pub fn sphere(&mut self, center: Vec3, radius: f32) -> NodeId {
        self.create_node(Node::Sphere { center, radius })
    }

    /// A box with rounded edges / corners.
    ///
    /// The rounding is subtracted from the edges and corners (sandpapered down).
    pub fn rounded_box(&mut self, half_size: Vec3, rounding_radius: f32) -> NodeId {
        self.create_node(Node::RoundedBox {
            half_size,
            rounding_radius,
        })
    }

    /// A ring, e.g. a donut.
    ///
    /// Centered at origin, lying in the XZ plane.
    pub fn torus(&mut self, big_r: f32, small_r: f32) -> NodeId {
        self.create_node(Node::Torus { big_r, small_r })
    }

    /// Part of a ring, e.g. a donut someone has taken a bite out of.
    ///
    /// Centered at origin, lying in the XZ plane.
    /// The missing piece of the torus is in the negative Z direction.
    ///
    /// If `half_angle=PI` you get a full torus, with `half_angle=PI/2` you get half a torus etc.
    pub fn torus_sector(&mut self, big_r: f32, small_r: f32, half_angle: f32) -> NodeId {
        self.create_node(Node::TorusSector {
            big_r,
            small_r,
            sin_cos_half_angle: half_angle.sin_cos(),
        })
    }

    /// A biconvex lens consisting of two circle segments with the
    /// same chord length and a lower and an upper sagitta.
    pub fn biconvex_lens(&mut self, lower_sagitta: f32, upper_sagitta: f32, chord: f32) -> NodeId {
        // Clamp inputs to avoid rendering artifacts:
        let min_sagitta = 1e-3;
        let max_sagitta = chord / 2.0;
        let upper_sagitta = upper_sagitta.clamp(min_sagitta, max_sagitta);
        let lower_sagitta = lower_sagitta.clamp(min_sagitta, max_sagitta);

        self.create_node(Node::BiconvexLens {
            lower_sagitta,
            upper_sagitta,
            chord,
        })
    }

    pub fn capsule(&mut self, points: [Vec3; 2], radius: f32) -> NodeId {
        self.create_node(Node::Capsule { points, radius })
    }

    /// A simple capsule from the origin along the Y axis.
    pub fn capsule_y(&mut self, length: f32, radius: f32) -> NodeId {
        self.create_node(Node::Capsule {
            points: [Vec3::ZERO, Vec3::new(0.0, length, 0.0)],
            radius,
        })
    }

    /// A cylinder with rounded edges.
    /// The cylinder has the center in origin, and stretches along the Y axis.
    ///
    /// The rounding is subtracted from the edges (sandpapered down).
    ///
    /// When `rounding_radius == 2 * cylinder_radius` you get a capsule.
    ///
    /// When `half_height = rounding_radius` you get a filled torus.
    pub fn rounded_cylinder(
        &mut self,
        cylinder_radius: f32,
        half_height: f32,
        rounding_radius: f32,
    ) -> NodeId {
        self.create_node(Node::RoundedCylinder {
            cylinder_radius,
            half_height,
            rounding_radius,
        })
    }

    /// The convex hull of two spheres.
    pub fn tapered_capsule(&mut self, points: [Vec3; 2], radii: [f32; 2]) -> NodeId {
        // The current SDF renderer freaks out in the degenerate case where one sphere fully contains the other, so
        // we special-case that here.
        // It would be cleaner to fix it in the shader (since this solution changes the structure of the graph),
        // but it would also make rendering slower.
        let distance = (points[0] - points[1]).length();
        if distance + radii[1] <= radii[0] {
            self.sphere(points[0], radii[0])
        } else if distance + radii[0] <= radii[1] {
            self.sphere(points[1], radii[1])
        } else {
            self.create_node(Node::TaperedCapsule { points, radii })
        }
    }

    /// Base center at origin, extending `height` along positive Y axis.
    pub fn cone(&mut self, radius: f32, height: f32) -> NodeId {
        self.create_node(Node::Cone { radius, height })
    }

    pub fn op_material(&mut self, child: NodeId, material: Material) -> NodeId {
        self.create_node(Node::Material { child, material })
    }

    pub fn op_rgb(&mut self, child: NodeId, rgb: impl Into<Vec3>) -> NodeId {
        self.op_material(child, Material::new(rgb.into()))
    }

    pub fn op_union(&mut self, lhs: NodeId, rhs: NodeId) -> NodeId {
        self.create_node(Node::Union { lhs, rhs })
    }

    pub fn op_union_smooth(&mut self, lhs: NodeId, rhs: NodeId, size: f32) -> NodeId {
        self.create_node(Node::UnionSmooth { lhs, rhs, size })
    }

    pub fn op_union_multi(&mut self, children: Vec<NodeId>) -> NodeId {
        self.create_node(Node::UnionMulti { children })
    }

    pub fn op_union_multi_smooth(&mut self, children: Vec<NodeId>, size: f32) -> NodeId {
        self.create_node(Node::UnionMultiSmooth { children, size })
    }

    pub fn op_subtract(&mut self, lhs: NodeId, rhs: NodeId) -> NodeId {
        self.create_node(Node::Subtract { lhs, rhs })
    }

    pub fn op_subtract_smooth(&mut self, lhs: NodeId, rhs: NodeId, size: f32) -> NodeId {
        self.create_node(Node::SubtractSmooth { lhs, rhs, size })
    }

    pub fn op_intersect(&mut self, lhs: NodeId, rhs: NodeId) -> NodeId {
        self.create_node(Node::Intersect { lhs, rhs })
    }

    pub fn op_intersect_smooth(&mut self, lhs: NodeId, rhs: NodeId, size: f32) -> NodeId {
        self.create_node(Node::IntersectSmooth { lhs, rhs, size })
    }

    pub fn op_csg(&mut self, lhs: NodeId, op: CsgOp, rhs: NodeId) -> NodeId {
        match op {
            CsgOp::Union => self.create_node(Node::Union { lhs, rhs }),
            CsgOp::Subtract => self.create_node(Node::Subtract { lhs, rhs }),
            CsgOp::Intersect => self.create_node(Node::Intersect { lhs, rhs }),
        }
    }

    pub fn op_csg_smooth(&mut self, lhs: NodeId, op: CsgOp, rhs: NodeId, size: f32) -> NodeId {
        match op {
            CsgOp::Union => self.create_node(Node::UnionSmooth { lhs, rhs, size }),
            CsgOp::Subtract => self.create_node(Node::SubtractSmooth { lhs, rhs, size }),
            CsgOp::Intersect => self.create_node(Node::IntersectSmooth { lhs, rhs, size }),
        }
    }

    pub fn op_rotate(&mut self, child: NodeId, rotation: impl Into<Quat>) -> NodeId {
        self.create_node(Node::Rotate {
            rotation: rotation.into(),
            child,
        })
    }

    pub fn op_translate(&mut self, child: NodeId, translation: impl Into<Vec3>) -> NodeId {
        self.create_node(Node::Translate {
            translation: translation.into(),
            child,
        })
    }

    pub fn op_scale(&mut self, child: NodeId, scale: impl Into<f32>) -> NodeId {
        self.create_node(Node::Scale {
            scale: scale.into(),
            child,
        })
    }

    pub fn op_iso_transform(
        &mut self,
        mut node: NodeId,
        transform: &crate::IsoTransform,
    ) -> NodeId {
        node = self.op_rotate(node, transform.rotation());
        node = self.op_translate(node, transform.translation());
        node
    }

    pub fn op_conformal3(&mut self, mut node: NodeId, transform: &crate::Conformal3) -> NodeId {
        node = self.op_scale(node, transform.scale());
        node = self.op_rotate(node, transform.rotation());
        node = self.op_translate(node, transform.translation());
        node
    }
}

// misc
impl Graph {
    pub fn get(&self, node_id: NodeId) -> Option<&Node> {
        self.nodes.get(&node_id)
    }

    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&node_id)
    }

    pub fn nodes(&self) -> impl Iterator<Item = (&NodeId, &Node)> {
        self.nodes.iter()
    }

    pub fn bounding_box(&self, node: NodeId) -> BoundingBox {
        let node = self.nodes.get(&node).unwrap();

        #[allow(clippy::match_same_arms)] // nicer to have the cases separately here, for now?
        match node {
            Node::Plane { .. } => BoundingBox::everything(),
            Node::Sphere { center, radius } => {
                BoundingBox::from_center_size(*center, Vec3::splat(2.0 * radius))
            }
            Node::Capsule { points, radius } => {
                let min = Vec3::new(
                    (points[0].x - radius).min(points[1].x - radius),
                    (points[0].y - radius).min(points[1].y - radius),
                    (points[0].z - radius).min(points[1].z - radius),
                );
                let max = Vec3::new(
                    (points[0].x + radius).max(points[1].x + radius),
                    (points[0].y + radius).max(points[1].y + radius),
                    (points[0].z + radius).max(points[1].z + radius),
                );
                BoundingBox::from_min_max(min, max)
            }
            Node::RoundedCylinder {
                cylinder_radius,
                half_height,
                ..
            } => BoundingBox::from_min_max(
                Vec3::new(-*cylinder_radius, -*half_height, -*cylinder_radius),
                Vec3::new(*cylinder_radius, *half_height, *cylinder_radius),
            ),
            Node::TaperedCapsule { points, radii } => {
                let min = Vec3::new(
                    (points[0].x - radii[0]).min(points[1].x - radii[1]),
                    (points[0].y - radii[0]).min(points[1].y - radii[1]),
                    (points[0].z - radii[0]).min(points[1].z - radii[1]),
                );
                let max = Vec3::new(
                    (points[0].x + radii[0]).max(points[1].x + radii[1]),
                    (points[0].y + radii[0]).max(points[1].y + radii[1]),
                    (points[0].z + radii[0]).max(points[1].z + radii[1]),
                );
                BoundingBox::from_min_max(min, max)
            }
            Node::Cone { radius, height } => BoundingBox::from_min_max(
                Vec3::new(-radius, 0.0, -radius),
                Vec3::new(*radius, *height, *radius),
            ),
            Node::RoundedBox { half_size, .. } => {
                BoundingBox::from_center_size(Vec3::ZERO, *half_size * 2.0)
            }
            Node::Torus { big_r, small_r } => BoundingBox::from_center_size(
                Vec3::ZERO,
                2.0 * Vec3::new(big_r + small_r, *small_r, big_r + small_r),
            ),
            Node::TorusSector {
                big_r,
                small_r,
                sin_cos_half_angle,
            } => {
                let big_r = *big_r;
                let small_r = *small_r;
                let (sin, cos) = *sin_cos_half_angle;
                if cos > 0.0 {
                    // Less than half a torus
                    let x = big_r * sin;
                    let z = big_r * cos;
                    BoundingBox::from_min_max(Vec3::new(-x, 0.0, z), Vec3::new(x, 0.0, big_r))
                } else {
                    // More than half a torus
                    let z = big_r * cos;
                    BoundingBox::from_min_max(
                        Vec3::new(-big_r, 0.0, z),
                        Vec3::new(big_r, 0.0, big_r),
                    )
                }
                .expanded(Vec3::splat(small_r))
            }
            Node::BiconvexLens {
                lower_sagitta,
                upper_sagitta,
                chord,
            } => {
                let chord_radius = chord / 2.0;
                BoundingBox::from_min_max(
                    Vec3::new(-chord_radius, -*lower_sagitta, -chord_radius),
                    Vec3::new(chord_radius, *upper_sagitta, chord_radius),
                )
            }

            Node::Material { child, .. } => self.bounding_box(*child),
            Node::Union { lhs, rhs } => self.bounding_box(*lhs).union(self.bounding_box(*rhs)),
            Node::UnionSmooth { lhs, rhs, size: _ } => {
                // The smooth union operator sometimes makes the surface
                // grow outside the bounding boxes of the parts, which is unfortunate.
                // So what we should do here is probably expand by `size`, but in practice
                // this is seldome needed and makes the bounding boxes a lot larger
                // TODO: Find a smooth union operator
                // that never grows outside the original bounding boxes?
                self.bounding_box(*lhs).union(self.bounding_box(*rhs))
                // .expanded(Vec3::splat(*size))
            }
            Node::UnionMulti { children } => {
                let mut bbox = BoundingBox::nothing();
                for child in children.iter() {
                    bbox = bbox.union(self.bounding_box(*child));
                }
                bbox
            }
            Node::UnionMultiSmooth { children, size: _ } => {
                let mut bbox = BoundingBox::nothing();
                for child in children.iter() {
                    bbox = bbox.union(self.bounding_box(*child));
                }
                bbox
            }
            Node::Subtract { lhs, .. } => self.bounding_box(*lhs),
            Node::SubtractSmooth { lhs, .. } => self.bounding_box(*lhs),
            Node::Intersect { lhs, rhs } => self
                .bounding_box(*lhs)
                .intersection(self.bounding_box(*rhs)),
            Node::IntersectSmooth { lhs, rhs, .. } => self
                .bounding_box(*lhs)
                .intersection(self.bounding_box(*rhs)),
            Node::Translate { translation, child } => {
                self.bounding_box(*child).translated(*translation)
            }
            Node::Rotate { rotation, child } => {
                self.bounding_box(*child).rotated_around_origin(rotation)
            }
            Node::Scale { scale, child } => {
                assert!(*scale >= 0.0, "TODO: prevent and/or support negative scale");
                let mut bbox = self.bounding_box(*child);
                bbox.min *= *scale;
                bbox.max *= *scale;
                bbox
            }
            Node::Graph { graph, root } => graph.bounding_box(*root),
        }
    }
}

/// Allows you to animate and play with the example scene.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
pub struct ExampleParams {
    pub big_r: f32,
    pub small_r: f32,
    pub height: f32,
    pub angle: f32,
    pub smoothness: f32,
    pub capsule_y: f32,
    pub rounding_radius: f32,
    pub box_scale: f32,
    pub box_translation_y: f32,
    pub biconvex_lens_lower_sagitta: f32,
    pub biconvex_lens_upper_sagitta: f32,
    pub biconvex_lens_chord: f32,
    pub head: f32,
}

impl Default for ExampleParams {
    fn default() -> Self {
        Self {
            big_r: 1.0,
            small_r: 0.5,
            height: 2.0,
            angle: std::f32::consts::TAU * 2.0 / 3.0,
            smoothness: 0.45,
            capsule_y: 0.3,
            rounding_radius: 0.3,
            box_scale: 0.5,
            box_translation_y: 0.5,
            biconvex_lens_lower_sagitta: 0.5,
            biconvex_lens_upper_sagitta: 0.3,
            biconvex_lens_chord: 1.0,
            head: 1.0,
        }
    }
}

impl Graph {
    pub fn example(&mut self, params: &ExampleParams) -> NodeId {
        // TODO: make use of all node types
        let example_operations = self.example_operations(params);

        let tapered_capsule = self.tapered_capsule(
            [Vec3::new(0.0, 0.0, 6.0), Vec3::new(0.0, params.height, 6.0)],
            [params.big_r, params.small_r],
        );

        let cone = self.cone(params.big_r, params.height);
        let cone = self.op_translate(cone, Vec3::new(0.0, 0.0, 9.0));

        let rounded_cylinder =
            self.rounded_cylinder(params.big_r, params.height / 2.0, params.rounding_radius);
        let rounded_cylinder =
            self.op_translate(rounded_cylinder, Vec3::new(3.0, params.height / 2.0, 6.0));

        let torus = self.torus(params.big_r, params.small_r);
        let torus = self.op_translate(torus, Vec3::new(-3.0, 0.0, 6.0));

        let torus_sector = self.torus_sector(params.big_r, params.small_r, params.angle / 2.0);
        let torus_sector = self.op_translate(torus_sector, Vec3::new(-3.0, 2.0, 6.0));

        self.op_union_multi(vec![
            example_operations,
            tapered_capsule,
            cone,
            rounded_cylinder,
            torus,
            torus_sector,
        ])
    }

    pub fn example_operations(&mut self, params: &ExampleParams) -> NodeId {
        let sphere = self.sphere(Vec3::new(0.0, 0.0, 0.0), 1.0);
        let sphere = self.op_rgb(sphere, Vec3::new(0.3, 0.7, 0.3));

        let capsule = self.capsule(
            [
                Vec3::new(-2.0, params.capsule_y, 0.0),
                Vec3::new(2.0, params.capsule_y, 0.0),
            ],
            0.65,
        );
        let capsule = self.op_rgb(capsule, Vec3::new(0.3, 0.3, 0.9));

        let rounded_box = self.rounded_box(Vec3::new(0.5, 1.0, 2.0), params.rounding_radius);
        let rounded_box = self.op_rgb(rounded_box, Vec3::new(1.0, 0.3, 0.9));
        let rounded_box = self.op_rotate(rounded_box, Quat::from_rotation_y(params.angle));
        let rounded_box = self.op_scale(rounded_box, params.box_scale);
        let rounded_box = self.op_translate(rounded_box, params.box_translation_y * Vec3::Y);

        let biconvex_lens = self.biconvex_lens(
            params.biconvex_lens_lower_sagitta,
            params.biconvex_lens_upper_sagitta,
            params.biconvex_lens_chord,
        );
        let head_sphere = self.sphere(Vec3::new(0.0, 0.0, 0.0), params.head);

        let mouth = self.op_translate(biconvex_lens, Vec3::new(1.0, 0.0, 0.0));
        let head = self.op_subtract(head_sphere, mouth);
        let union_sharp = self.op_union(sphere, capsule);
        let subtract_sharp = self.op_subtract(sphere, capsule);
        let intersect_sharp = self.op_intersect(sphere, capsule);
        let union_smooth = self.op_union_smooth(sphere, capsule, params.smoothness);
        let subtract_smooth = self.op_subtract_smooth(sphere, capsule, params.smoothness);
        let intersect_smooth = self.op_intersect_smooth(sphere, capsule, params.smoothness);

        let nodes = vec![
            self.op_translate(union_sharp, Vec3::new(-3.0, 2.0, -3.0)),
            self.op_translate(subtract_sharp, Vec3::new(-3.0, 2.0, 0.0)),
            self.op_translate(intersect_sharp, Vec3::new(-3.0, 2.0, 3.0)),
            self.op_translate(union_smooth, Vec3::new(3.0, 2.0, -3.0)),
            self.op_translate(subtract_smooth, Vec3::new(3.0, 2.0, 0.0)),
            self.op_translate(intersect_smooth, Vec3::new(3.0, 2.0, 3.0)),
            self.op_translate(rounded_box, Vec3::new(0.0, 2.0, 0.0)),
            self.op_translate(head, Vec3::new(0.0, 2.0, 3.0)),
        ];
        self.op_union_multi(nodes)
    }
}
