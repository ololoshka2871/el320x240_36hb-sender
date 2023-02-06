// Текстура видео с камеры
// из текстуры читаются u8 но в шейдере они u32
@group(0) @binding(0) var cam_data: texture_2d<f32>;

// Сэмплер камеры
@group(0) @binding(1) var s_cam: sampler;

// Dithering matrix
@group(0) @binding(2) var<storage, read> dithering_matrix: array<u32>;

// Output binary image
//@group(0) @binding(3) var<storage, read_write> output_data: array<u32>;

// Output texture
// см таблицу https://gpuweb.github.io/gpuweb/#plain-color-formats какие форматы поддерживаются
@group(1) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;

//------------------------------------------------------------

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

@compute
@workgroup_size(16, 16) // размер рабочей группы, так как он двумерный, и в global_id будут двумерные координаты
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    var point_coords = vec2<u32>(global_id.x, global_id.y); // координаты пикселя который будем обрабатывать
    var tex_coords = vec2<f32>(point_coords) / vec2<f32>(textureDimensions(output_texture));
    var gray = textureSampleLevel(cam_data, s_cam, tex_coords, 0.0); // textureSamp() не разрешено в compute шейдерах
    var res = dither(vec2<f32>(point_coords), gray.r);

    // write to output texture 
    textureStore(output_texture, vec2<i32>(i32(point_coords.x), i32(point_coords.y)), vec4<f32>(res, 0.0, 0.0, 1.0));

    // todo: write to output binary image
}