use crate::sphere_tracing::ClosestHit;
use crate::sphere_tracing::Options;
use crate::Graph;
use crate::NodeId;
use macaw::Ray3;
use macaw::Vec3;

/// Marches a ray from `t_range.start()` until `t_range.end()`,
/// returning the first hit, or the place where the trace got closest to the surface.
pub fn march(
    graph: &Graph,
    root: NodeId,
    ray: Ray3,
    t_range: std::ops::RangeInclusive<f32>,
    opt: &Options,
) -> ClosestHit {
    // we could use the bounding box as an optimization to limit `t_range`
    let program = crate::compile(graph, root);
    let mut sd = to_sd_func(&program);
    crate::sphere_tracing::trace(&mut sd, ray, t_range, opt)
}

pub fn to_sd_func(program: &crate::Program) -> impl FnMut(Vec3) -> f32 + '_ {
    let mut d_context = crate::Interpreter::new_context(&program.opcodes, &program.constants);
    move |pos: Vec3| crate::Interpreter::<f32>::interpret(&mut d_context, pos).unwrap()
}
