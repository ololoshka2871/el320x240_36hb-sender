struct ComputeConfig {
    @location(0) width: u32,
};

// Текстура видео с камеры
// из текстуры читаются u8 но в шейдере они u32
@group(0) @binding(0) var cam_data: texture_2d<f32>;

// Сэмплер камеры
@group(0) @binding(1) var s_cam: sampler;

// Dithering matrix
@group(0) @binding(2) var<storage, read> dithering_matrix: array<u32>;

// Output binary image
// u8 не поддерживается, поэтому u32, за 1 вызов шейдера будет обрабатываться 32 ч/б пикселя
@group(0) @binding(3) var<storage, read_write> output_data: array<u32>;

// Config
@group(0) @binding(4) var<storage, read> config: ComputeConfig;

// Output texture
// см таблицу https://gpuweb.github.io/gpuweb/#plain-color-formats какие форматы поддерживаются
@group(1) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;

//------------------------------------------------------------

fn indexValue(position: vec2<f32>, matrix_size: f32, matrix_dim: u32) -> f32 {
    var x = u32(position.x) % matrix_dim;
    var y = u32(position.y) % matrix_dim;

    return f32(dithering_matrix[x + y * matrix_dim]) / matrix_size;
}

fn dither(position: vec2<f32>, color: f32, matrix_size: f32, matrix_dim: u32) -> f32 {
    var closestColor: f32;
    if color < 0.5 { closestColor = 0.0; } else { closestColor = 1.0; };
    var secondClosestColor = 1.0 - closestColor;
    var d = indexValue(position, matrix_size, matrix_dim);
    var distance = abs(closestColor - color);
    if distance < d { return closestColor; } else { return secondClosestColor; };
}

@compute
@workgroup_size(1) // В шейдер будем передавать координаты в плоском виде, поэтому размер группы 1
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let start_pixel = global_id.x * 32u; // 32 пикселя обрабатывается за 1 вызов шейдера
    let output_tex_dim = vec2<f32>(textureDimensions(output_texture));

    let matrix_size = f32(arrayLength(&dithering_matrix));
    let matrix_dim = u32(sqrt(matrix_size));

    // ounput pixel chank
    var output_u32 = 0u;

    for (var i = 0u; i < 32u; i += 1u) {
        let pixel = start_pixel + i;
        let point_coords = vec2<u32>(pixel % config.width, pixel / config.width); // координаты пикселя который будем обрабатывать

        let tex_coords = vec2<f32>(point_coords) / output_tex_dim;
        let gray = textureSampleLevel(cam_data, s_cam, tex_coords, 0.0); // textureSamp() не разрешено в compute шейдерах
        let res = dither(vec2<f32>(point_coords), gray.r, matrix_size, matrix_dim);

        // write to output texture 
        textureStore(output_texture, vec2<i32>(i32(point_coords.x), i32(point_coords.y)), vec4<f32>(res, 0.0, 0.0, 1.0));

        // write to output pixel if white
        if res == 1.0 {
            output_u32 |= (1u << (7u - (i % 8u))) << ((i / 8u) * 8u);
        }
    }

    // write to output pixel chank
    output_data[global_id.x] = output_u32;
}