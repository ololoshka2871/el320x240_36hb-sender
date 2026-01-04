use anyhow::*;
use image::GenericImageView;

// This is a struct that holds a texture, a texture view, and a sampler for that texture
#[allow(unused)]
pub(crate) struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    #[allow(unused)]
    pub(crate) fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?; // Load the image from bytes located in memory
        Self::from_image(device, queue, &img, Some(label))
    }

    pub(crate) fn empty(
        device: &wgpu::Device,
        dimensions: (u32, u32),
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        // Это типо дескриптора текстуры, адаптер для того чтобы делать запросы к драйверу
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(format!("{label}, view").as_str()),
            format: Some(format),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None,
        });
        // Сэмплер - это конвертер текстурная координата -> цвет пикселя
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, // Фильтр для увеличения, что делать если камера близко и надо вернуть цвет между физическими пикселями текстуры
            min_filter: wgpu::FilterMode::Nearest, // Фильтр для уменьшения, что делать если камера далеко и в 1 фрагмент попадает сразу много физических пикселей текстуры
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    #[allow(unused)]
    pub(crate) fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8(); // decode the image into a vector of pixels rgba format
        let dimensions = img.dimensions();

        // Создаем место под текстуру в памяти GPU
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, // Формат текстуры
            usage: wgpu::TextureUsages::TEXTURE_BINDING // TEXTURE_BINDING Шейдер сможет работать с данной текстурой
                | wgpu::TextureUsages::COPY_DST, // COPY_DST В данную текстуру можно будет записать данные
            view_formats: &[], // We don't need to create a texture view for this texture
        });

        // load data from diffuse_rgba to diffuse_texture allocated in GPU memory above
        queue.write_texture(
            // Куда копировать данные
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            // Источник данных
            &rgba,
            // Как копировать данные, преобразования форматов, например
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size, // размер текстуры
        );

        // Это типо дескриптора текстуры, адаптер для того чтобы делать запросы к драйверу
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Сэмплер - это конвертер текстурная координата -> цвет пикселя
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear, // Фильтр для увеличения, что делать если камера близко и надо вернуть цвет между физическими пикселями текстуры
            min_filter: wgpu::FilterMode::Nearest, // Фильтр для уменьшения, что делать если камера далеко и в 1 фрагмент попадает сразу много физических пикселей текстуры
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
