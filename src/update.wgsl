struct Particle {
  position: vec2<f32>,
  velocity: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> particles_src : array<Particle>;
@group(0) @binding(1) var<storage, read_write> particles_dst : array<Particle>;

fn hash(n: u32) -> f32 {
  var x = n;
  x = (x ^ 61u) ^ (x >> 16u);
  x = x * 9u;
  x = x ^ (x >> 4u);
  x = x * 0x27d4eb2du;
  x = x ^ (x >> 15u);
  return f32(x) / f32(0xffffffffu);
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  let total = arrayLength(&particles_src);
  let index = global_invocation_id.x;

  if (index >= total) {
    return;
  }

  var p = particles_src[index];

  if (length(p.velocity) == 0.0) {
    let r1 = hash(index * 1973u) * 2.0 - 1.0; // range [-1, 1]
    let r2 = hash(index * 9283u) * 2.0 - 1.0; // range [-1, 1]
    
    p.velocity = normalize(vec2<f32>(r1, r2)) * (hash(index * 1337u) * 2.0 + 1.0) / 10;
  }

  p.position += p.velocity * 0.001;

  if (p.position.x < -1 || p.position.x > 1) {
    p.velocity.x *= -1;
  }
  if (p.position.y < -1 || p.position.y > 1) {
    p.velocity.y *= -1;
  }
  particles_dst[index] = p;
}
