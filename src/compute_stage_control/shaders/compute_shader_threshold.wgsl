// No dithering, just black and white via average gray level betwen black_lvl and white_lvl

struct ComputeConfig {
    @location(0) width: u32,
    @location(1) threshold: f32,
};

// Текстура видео с камеры
// из текстуры читаются u8 но в шейдере они u32
@group(0) @binding(0) var cam_data: texture_2d<f32>;

// Сэмплер камеры
@group(0) @binding(1) var s_cam: sampler;

// Output binary image
// u8 не поддерживается, поэтому u32, за 1 вызов шейдера будет обрабатываться 32 ч/б пикселя
@group(0) @binding(2) var<storage, read_write> output_data: array<u32>;

// Config
@group(0) @binding(3) var<storage, read> config: ComputeConfig;

// Output texture
// см таблицу https://gpuweb.github.io/gpuweb/#plain-color-formats какие форматы поддерживаются
@group(1) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, write>;

//------------------------------------------------------------

@compute
@workgroup_size(1) // В шейдер будем передавать координаты в плоском виде, поэтому размер группы 1
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let start_pixel = global_id.x * 32u; // 32 пикселя обрабатывается за 1 вызов шейдера
    let output_tex_dim = vec2<f32>(textureDimensions(output_texture));

    // ounput pixel chank
    var output_u32 = 0u;

    for (var i = 0u; i < 32u; i += 1u) {
        let pixel = start_pixel + i;
        let point_coords = vec2<u32>(pixel % config.width, pixel / config.width); // координаты пикселя который будем обрабатывать

        let tex_coords = vec2<f32>(point_coords) / output_tex_dim;
        let gray = textureSampleLevel(cam_data, s_cam, tex_coords, 0.0); // textureSamp() не разрешено в compute шейдерах

        var res: f32;
        if gray.r < config.threshold {
            res = 0.0;
        } else { 
            res = 1.0;

            // write to output pixel if white
            output_u32 |= (1u << (7u - (i % 8u))) << ((i / 8u) * 8u);
        };

        // write to output texture 
        textureStore(output_texture, vec2<i32>(i32(point_coords.x), i32(point_coords.y)), vec4<f32>(res, 0.0, 0.0, 1.0));
    }

    // write to output pixel chank
    output_data[global_id.x] = output_u32;
}