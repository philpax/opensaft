//! Signed distance field function compiler/interpreter/discretizer/mesher.

// crate-specific exceptions:
#![forbid(unsafe_code)]
#![allow(
    clippy::enum_glob_use,      // TODO: Add? Used a lot on the opcodes
)]

use glam::Vec3;

pub use saft_sdf::*;

mod program;
pub use program::*;

mod compiler;
pub use compiler::*;

mod graph;
pub use graph::*;

mod grid3;
pub use grid3::*;

mod mesh;
pub use mesh::*;

mod marching_cubes;
pub use marching_cubes::*;

pub mod sphere_tracing;

mod trace;
pub use trace::*;

mod codegen;
pub use codegen::*;

mod math;
pub use math::*;

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
pub struct MeshOptions {
    /// Desired mean resolution on the axis.
    ///
    /// the total number of grid points will be close to resolution ^ 3
    pub mean_resolution: f32,

    /// Trying to fit the number of cubes to the mean resolution can lead to some
    /// extreme cases if the box is very narrow. Use these parameters to clamp
    /// the resolution to desired "sane" bounds.
    pub max_resolution: f32,
    pub min_resolution: f32,
}

impl MeshOptions {
    pub fn low() -> Self {
        Self {
            mean_resolution: 32.0,
            max_resolution: 64.0,
            min_resolution: 8.0,
        }
    }
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self {
            mean_resolution: 64.0,
            max_resolution: 128.0,
            min_resolution: 8.0,
        }
    }
}

pub fn transform_positions_in_place(
    mesh: &mut TriangleMesh,
    world_from_grid_f: impl Fn(Vec3) -> Vec3 + Send + Sync,
) {
    #[cfg(feature = "with_rayon")]
    {
        use rayon::prelude::*;

        mesh.positions.par_iter_mut().for_each(|p| {
            // transform to world:
            *p = world_from_grid_f((*p).into()).into();
        });
    }

    #[cfg(not(feature = "with_rayon"))]
    {
        mesh.positions.iter_mut().for_each(|p| {
            // transform to world:
            *p = world_from_grid_f((*p).into()).into();
        });
    }
}

pub fn gather_colors_in_place(
    mesh: &mut TriangleMesh,
    color_world: impl Fn(Vec3) -> Vec3 + Send + Sync,
) {
    #[cfg(feature = "with_rayon")]
    {
        use rayon::prelude::*;

        mesh.colors = mesh
            .positions
            .par_iter()
            .map(|p| color_world(Vec3::new(p[0], p[1], p[2])).into())
            .collect();
    }

    #[cfg(not(feature = "with_rayon"))]
    {
        mesh.colors = mesh
            .positions
            .iter()
            .map(|p| color_world(Vec3::new(p[0], p[1], p[2])).into())
            .collect();
    }
}

pub fn mesh_from_sdf_func(
    bb: &BoundingBox,
    resolution: [usize; 3],
    sd_world: impl Fn(Vec3) -> f32 + Send + Sync,
    color_world: impl Fn(Vec3) -> Vec3 + Send + Sync,
) -> Result<TriangleMesh, Error> {
    use glam::*;

    let world_from_grid_scale = bb.size().x / (resolution[0] as f32 - 1.0);
    let grid_from_world_scale = 1.0 / world_from_grid_scale;

    let world_from_grid_f = |pos_in_grid: Vec3| bb.min + world_from_grid_scale * pos_in_grid;

    let world_from_grid_i = |pos_in_grid: Index3| {
        let pos_in_grid = Vec3::new(
            pos_in_grid[0] as f32,
            pos_in_grid[1] as f32,
            pos_in_grid[2] as f32,
        );
        world_from_grid_f(pos_in_grid)
    };

    let sd_in_grid = |pos_in_grid| {
        let pos_in_world = world_from_grid_i(pos_in_grid);
        grid_from_world_scale * sd_world(pos_in_world)
    };

    let mut grid = Grid3::<f32>::new(resolution);
    grid.set_truncated(sd_in_grid, 2.0);

    // Check a single sample for NaN. Often a NaN will end up in the whole grid, so this'll catch it.
    if !grid.data()[grid.data().len() / 2].is_finite() {
        return Err(Error::EvaluatedToNaN);
    }

    let mut mesh = grid.marching_cubes();

    transform_positions_in_place(&mut mesh, world_from_grid_f);
    gather_colors_in_place(&mut mesh, color_world);

    Ok(mesh)
}

pub fn mesh_from_sdf_program(
    program: &Program,
    bb: &BoundingBox,
    resolution: [usize; 3],
) -> Result<TriangleMesh, Error> {
    let color_func = |pos_in_world| {
        let mut rgbd_context = Interpreter::new_context(&program.opcodes, &program.constants);
        Interpreter::<RgbWithDistance>::interpret(&mut rgbd_context, pos_in_world)
            .unwrap()
            .material()
            .rgb()
    };

    let d_func = |pos_in_world| {
        let mut d_context = Interpreter::new_context(&program.opcodes, &program.constants);
        Interpreter::<f32>::interpret(&mut d_context, pos_in_world)
            .unwrap()
            .distance()
    };

    mesh_from_sdf_func(bb, resolution, d_func, color_func)
}

pub fn mesh_from_sdf(
    graph: &Graph,
    node: NodeId,
    options: MeshOptions,
) -> Result<TriangleMesh, Error> {
    let (bb, resolution) = sdf_bb_and_resolution(graph.bounding_box(node), options);
    let program = compile(graph, node);

    mesh_from_sdf_program(&program, &bb, resolution)
}

/// Pick a good expanded bounding box and grid size from the given tight bounding box
pub fn sdf_bb_and_resolution(bb: BoundingBox, options: MeshOptions) -> (BoundingBox, [usize; 3]) {
    assert!(bb.is_finite(), "Bad saft bounding box: {:?}", bb);
    assert!(bb.volume() > 0.0, "Bad saft bounding box: {:?}", bb);

    // Add at least this many grid points on each side
    let grid_padding = 1.0;

    // preliminary so we can pad
    let grid_from_world_scale = options.mean_resolution / bb.volume().cbrt();
    let padding = grid_padding / grid_from_world_scale;
    let bb = bb.expanded(Vec3::splat(padding));

    // now actual:
    let grid_from_world_scale = options.mean_resolution / bb.volume().cbrt();

    let resolution = [
        grid_from_world_scale * bb.size().x,
        grid_from_world_scale * bb.size().y,
        grid_from_world_scale * bb.size().z,
    ];

    let max_side = resolution[0].max(resolution[1]).max(resolution[2]);
    let max_factor = if max_side > options.max_resolution {
        options.max_resolution / max_side
    } else {
        1.0
    };
    let min_side = resolution[0].min(resolution[1]).min(resolution[2]);
    let min_factor = if min_side < options.min_resolution {
        options.min_resolution / min_side
    } else {
        1.0
    };

    // Let the minimum overrule the maximum.
    let factor = min_factor.max(max_factor);

    let grid_resolution = [
        (factor * resolution[0]).ceil() as usize,
        (factor * resolution[1]).ceil() as usize,
        (factor * resolution[2]).ceil() as usize,
    ];

    /*
    // Useful for debugging the above calculations. Turns out it's not as intuitive as expected to get it right.
    println!(
        "max_res: {} min_res: {} max_factor: {} min_factor: {} original_resolution: {:?} grid_resolution: {:?}",
        options.max_resolution, options.min_resolution, max_factor, min_factor, resolution, grid_resolution
    );
    */

    (bb, grid_resolution)
}

pub fn surface_distance_to(graph: &Graph, node: NodeId, pos: Vec3) -> f32 {
    let program = compile(graph, node);
    let mut d_context = Interpreter::new_context(&program.opcodes, &program.constants);
    Interpreter::<f32>::interpret(&mut d_context, pos).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut graph = Graph::default();
        let node = graph.sphere(Vec3::default(), 1.0);

        let node2 = graph.sphere(Vec3::default(), 0.5);
        let node3 = graph.op_translate(node2, -Vec3::X);
        let node4 = graph.op_translate(node2, Vec3::X);
        let node5 = graph.op_translate(node2, Vec3::Y);

        let union_node = graph.op_union_multi(vec![node, node3, node4, node5]);

        let program = compile(&graph, union_node);
        let mut grid = Grid3::<f32>::new([64, 64, 64]);
        grid.set_truncated_sync(
            |pos| {
                let mut context = Interpreter::new_context(&program.opcodes, &program.constants);

                let pos = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
                Interpreter::<f32>::interpret(&mut context, pos).unwrap()
            },
            2.0,
        );

        let mut grid2 = Grid3::new([64, 64, 64]);
        grid2.set_truncated(
            |pos| {
                let pos = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
                sd_op_union(
                    sd_sphere(pos, Vec3::default(), 1.0),
                    sd_op_union(
                        sd_sphere(pos + Vec3::X, Vec3::default(), 0.5),
                        sd_op_union(
                            sd_sphere(pos - Vec3::X, Vec3::default(), 0.5),
                            sd_sphere(pos - Vec3::Y, Vec3::default(), 0.5),
                        ),
                    ),
                )
            },
            2.0,
        );

        // grid and grid2 should be equal.
        assert!(grid == grid2);
    }
}
