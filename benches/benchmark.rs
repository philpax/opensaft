#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ok to use in tests, benches & build scripts

use tiny_bench::BenchmarkConfig;

pub fn main() {
    let mut graph = saft::Graph::default();
    let root = graph.example(&Default::default());

    let mesh_options = saft::MeshOptions {
        mean_resolution: 128.0,
        max_resolution: 128.0,
        min_resolution: 8.0,
    };
    let mesh = saft::mesh_from_sdf(&graph, root, mesh_options).unwrap();
    eprintln!(
        "{:.1}k vertices and {:.1}k triangles",
        mesh.positions.len() as f32 * 1e-3,
        (mesh.indices.len() / 3) as f32 * 1e-3
    );

    let bench_cfg = BenchmarkConfig {
        num_samples: 10,
        ..Default::default()
    };
    tiny_bench::bench_with_configuration_labeled("mech_from_sdf", &bench_cfg, || {
        saft::mesh_from_sdf(&graph, root, mesh_options)
    });
}
