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

// diffusion matrix
@group(0) @binding(4) var<storage, read> dithering_matrix: array<u32>;

fn indexValue(position: vec2<f32>) -> f32 {
    let matrix_size = arrayLength(&dithering_matrix);
    let matrix_dim = u32(sqrt(f32(matrix_size)));

    var x = u32(position.x) % matrix_dim;
    var y = u32(position.y) % matrix_dim;

    return f32(dithering_matrix[x + y * matrix_dim]) / f32(matrix_size);
}

fn dither(position: vec2<f32>, color: f32) -> f32 {
    var closestColor: f32;
    if color < 0.5 { closestColor = 0.0; } else { closestColor = 1.0; };
    var secondClosestColor = 1.0 - closestColor;
    var d = indexValue(position);
    var distance = abs(closestColor - color);
    if distance < d { return closestColor; } else { return secondClosestColor; };
}

// Mark the entry point as posible entry point for the fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var gray = textureSample(cam_diffuse, s_cam, in.tex_coords);
    var res = dither(vec2<f32>(in.clip_position.x, in.clip_position.y), gray.r);
    return vec4<f32>(res, res, res, 1.0);
}