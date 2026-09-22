struct Particle {
  position: vec2<f32>,
  velocity: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> particles_src : array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_dst : array<Particle>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

}
