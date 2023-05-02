use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use crate::texture::Texture;

// rust code representation of the ComputeConfig struct in the compute shader
#[repr(C)]
// Не дать rust переставить поля местами, будет как в C и как в шейдере
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Config {
    width: u32,
    height: u32,
    threshold: f32,
}

pub(crate) struct PinwheelComputeStageControl {
    output_size: PhysicalSize<u32>,
}

impl PinwheelComputeStageControl {
    pub(crate) fn new(output_size: PhysicalSize<u32>) -> Self {
        Self { output_size }
    }
}

impl super::ComputeStageControl for PinwheelComputeStageControl {
    fn configure(
        &self,
        device: &wgpu::Device,
        camera_texture: &Texture,
        display_textures: &[Texture],
        lvls: (f32, f32),
    ) -> (
        wgpu::ComputePipeline,
        wgpu::Buffer,
        wgpu::BindGroup,
        Vec<wgpu::BindGroup>,
        wgpu::Buffer,
    ) {
        todo!();
        // Пустой буффер для входных данных
        let output_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output data"),
            size: (self.output_size.height * self.output_size.width / 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false, // этот буфер вообще не мапить!
        });

        // Пустой промежуточный буффер для выходных данных
        let output_copy_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output data copy"),
            size: output_data_buffer.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false, // этот мапить по запросу
        });

        // Буфер содержащий конфигурацию
        let config_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "config buffer".into(),
            contents: bytemuck::cast_slice(&[Config {
                width: self.output_size.width,
                height: self.output_size.height,
                threshold: (lvls.1 - lvls.0) / 2.0,
            }]),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Создание сэмплера для камеры
        let camera_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Camera sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Загрузка вычислительного шейдера
        let cs_module = device
            .create_shader_module(wgpu::include_wgsl!("shaders/compute_shader_pinwheel.wgsl"));

        // Создание группы привязки постоянных данных для вычислительного шейдера
        let cs_const_biding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute shader const binding layout"),
                entries: &[
                    // Входные данные c камеры
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Сэмплер для входной текстуры
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // Выходные данные
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Конфигурация
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Создание группы привязки выходных данных для вычислительного шейдера (тут будет меняться текстура из display_textures)
        let cs_output_buffered_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute shader output binding layout"),
                entries: &[
                    // Выходная текстура после дизеринга
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadWrite,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        // Загрузка байндигов для вычислительного шейдера
        let cs_const_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute shader const binding"),
            layout: &cs_const_biding_layout,
            entries: &[
                // Входные данные c камеры
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&camera_texture.view),
                },
                // Сэмплер для входной текстуры
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&camera_sampler),
                },
                // Выходные данные
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &output_data_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(output_data_buffer.size()),
                    }),
                },
                // Конфигурация
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &config_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(config_buffer.size()),
                    }),
                },
            ],
        });

        // Загрузка байндигов для вычислительного шейдера (будет передаваться поочередно в каждый кадр)
        let cs_output_buffered_bindings = display_textures
            .iter()
            .enumerate()
            .map(|(i, t)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&format!("Display texture {}", i)),
                    layout: &cs_output_buffered_binding_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&t.view),
                    }],
                })
            })
            .collect::<Vec<_>>();

        // Создание лайаута пайплайна вычислительного шейдера
        let cs_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Compute shader pipeline layout"),
            bind_group_layouts: &[&cs_const_biding_layout, &cs_output_buffered_binding_layout],
            push_constant_ranges: &[],
        });

        // Создание пайплайна вычислительного шейдера
        let cs_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute shader pipeline"),
            layout: Some(&cs_pipeline_layout),
            module: &cs_module,
            entry_point: "main",
        });

        (
            cs_pipeline,
            output_data_buffer,
            cs_const_binding,
            cs_output_buffered_bindings,
            output_copy_buffer,
        )
    }

    fn call_dispatch(&self, cs: &mut wgpu::ComputePass) {
        cs.dispatch_workgroups((self.output_size.width * self.output_size.height) / 8, 1, 1);
    }
}
