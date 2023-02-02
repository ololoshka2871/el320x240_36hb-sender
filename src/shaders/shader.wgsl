struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

// Mark the entry point as posible entry point for the vertex shader
@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;

    // add Z coordinate = 0 and W (transformation required) coordinate = 1
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Текстура дерева
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;

// Текстура видео с камеры
// из текстуры читаются u8 но в шейдере они u32
@group(0) @binding(1)
var cam_diffuse: texture_2d<f32>;

// Сэмплер дерева
@group(0) @binding(2)
var s_diffuse: sampler;

// Сэмплер камеры
@group(0) @binding(3)
var s_cam: sampler;

// Mark the entry point as posible entry point for the fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var gray = textureSample(cam_diffuse, s_cam, in.tex_coords);
    return gray;
    //return vec4<f32>(gray.x, gray.x, gray.x, 1.0);
    //return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}