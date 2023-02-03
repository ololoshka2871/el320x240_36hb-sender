struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

//-----------------------------------------------------------------------------

// Mark the entry point as posible entry point for the vertex shader
@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;

    // add Z coordinate = 0 and W (transformation required) coordinate = 1
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

//-----------------------------------------------------------------------------

// output texture
@group(0) @binding(0) var output_texture: texture_2d<f32>;

// sampler
@group(1) @binding(0) var o_sampler: sampler;

// Mark the entry point as posible entry point for the fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let gray = textureSample(output_texture, o_sampler, in.tex_coords);
    return vec4<f32>(gray.r, gray.r, gray.r, 1.0);
}