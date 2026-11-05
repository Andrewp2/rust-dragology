struct VertexOut {
    @builtin(position) pos: vec4f,
    @location(0) local: vec2f,
    @location(1) half_size: vec2f,
    @location(2) radius: f32,
    @location(3) color: vec4f,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32,
          @location(0) center: vec2f, @location(1) half_clip: vec2f,
          @location(2) half_size: vec2f, @location(3) radius: f32,
          @location(4) color: vec4f) -> VertexOut {
    let corners = array(vec2f(-1,-1), vec2f(1,-1), vec2f(-1,1), vec2f(1,1));
    let corner = corners[index];
    return VertexOut(vec4f(center + corner * half_clip, 0, 1),
                     corner * half_size, half_size, radius, color);
}

@fragment
fn fragment(v: VertexOut) -> @location(0) vec4f {
    let q = abs(v.local) - v.half_size + v.radius;
    let distance = length(max(q, vec2f(0))) + min(max(q.x,q.y),0) - v.radius;
    let coverage = clamp(0.5 - distance / max(fwidth(distance), 0.001), 0, 1);
    return vec4f(v.color.rgb, v.color.a * coverage);
}
