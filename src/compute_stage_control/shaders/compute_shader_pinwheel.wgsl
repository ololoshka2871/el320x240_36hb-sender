// Pinwheel filer: https://escholarship.org/content/qt7b78v752/qt7b78v752_noSplash_b7b84686bf8195e832c0afa9e46c633e.pdf

struct ComputeConfig {
    @location(0) width: u32,
    @location(1) height: u32,
    @location(2) threshold: f32,
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
@group(1) @binding(0) var output_texture: texture_storage_2d<rgba8unorm, read_write>;

//------------------------------------------------------------

@compute
@workgroup_size(1) // В шейдер будем передавать координаты в плоском виде, поэтому размер группы 1
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let step_types: array<u32, 16> = array<u32, 16>(
        0u,
        1u,
        2u, 2u,
        3u, 3u,
        0u, 0u, 0u,
        1u, 1u, 1u,
        2u, 2u, 2u, 2u,
    );

    let a = 7.0 / 16.0;
    let b = 3.0 / 16.0;
    let c = 5.0 / 16.0;
    let d = 1.0 / 16.0;
    
    let matrixes = array<mat3x3<f32>, 4>(
        // down
        mat3x3<f32>(
            0.0, 0.0, b,
            0.0, 0.0, c,
            0.0, a, d,
        ),
        // left
        mat3x3<f32>(
            0.0, 0.0, 0.0,
            a, 0.0, 0.0,
            d, c, b,
        ),
        // up
        mat3x3<f32>(
            d, a, 0.0,
            c, 0.0, 0.0,
            b, 0.0, 0.0,
        ),
        // right
        mat3x3<f32>(
            d, c, b,
            0.0, 0.0, a,
            0.0, 0.0, 0.0,
        ),
    );

    let blocks_per_x = config.width / (4u + 4u);

    let block_num = global_id.x;

    // Начальная точка спирали
    var x = 3u + ((4u + 4u) * block_num) % blocks_per_x;
    var y = 1u + block_num / blocks_per_x;

    let output_tex_dim = vec2<f32>(textureDimensions(output_texture));

    // fill output block with gray pixels
    {
        var start_point = vec2<u32>(x - 2u, y - 1u);
        for (var x_add = 0u; x_add < 4u; x_add += 1u) {
            for (var y_add = 0u; y_add < 4u; y_add += 1u) {
                let point_coords = start_point + vec2<u32>(x_add, y_add);

                let tex_coords = vec2<f32>(point_coords) / output_tex_dim;
                let gray = textureSampleLevel(cam_data, s_cam, tex_coords, 0.0); // textureSamp() не разрешено в compute шейдерах

                // write to output texture 
                textureStore(output_texture, vec2<i32>(i32(point_coords.x), i32(point_coords.y)), vec4<f32>(gray.r, 0.0, 0.0, 1.0));
            }
        }
    }

    // output pixel chank
    var output_u32 = 0u;

    //for (var i = 0u; i < (4u * 4u); i += 1u) {
    //    let matrix_idx = step_types[i];
    //    let matrix = matrixes[matrix_idx];
    //
    //    let point_coords = vec2<u32>(x, y);
    //
    //    dither(point_coords, matrix);
    //
    //    // переход к следующему пикселю
    //    switch matrix_idx {
    //        case 0u: {
    //            y += 1u;
    //        }
    //        case 1u: {
    //            x -= 1u;
    //        }
    //        case 2u: {
    //            y -= 1u;
    //        }
    //        case 3u: {
    //            x += 1u;
    //        }
    //    }
    //}

    // write to output pixel chank
    output_data[global_id.x] = output_u32;
}