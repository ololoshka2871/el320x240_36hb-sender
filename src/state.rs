use core::default::Default;
use std::iter;

use nokhwa::pixel_format;
use wgpu::{util::DeviceExt, BindGroup, Buffer};
use winit::{dpi::PhysicalSize, event::*, window::Window};

use crate::texture::Texture;

// if feature diffuse8x8 is enabled use duffusion_matrix 8x8 elese use 4x4
#[cfg(not(feature = "diffuse8x8"))]
const DITHERING_MATRIX: [u32; 16] = [
    0u32, 8, 2, 10, // row 0
    12, 4, 14, 6, // row 1
    3, 11, 1, 9, // row 2
    15, 7, 13, 5, // row 3
];

#[cfg(feature = "diffuse8x8")]
const DITHERING_MATRIX: [u32; 64] = [
    0, 32, 8, 40, 2, 34, 10, 42, // row 0
    48, 16, 56, 24, 50, 18, 58, 26, // row 1
    12, 44, 4, 36, 14, 46, 6, 38, // row 2
    60, 28, 52, 20, 62, 30, 54, 22, // row 3
    3, 35, 11, 43, 1, 33, 9, 41, // row 4
    51, 19, 59, 27, 49, 17, 57, 25, // row 5
    15, 47, 7, 39, 13, 45, 5, 37, // row 6
    63, 31, 55, 23, 61, 29, 53, 21, // row 7
];

pub(crate) struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate) window_size: winit::dpi::PhysicalSize<u32>,
    window: Window,
    output_size: PhysicalSize<u32>,

    camera_texture: crate::texture::Texture,
    camera: nokhwa::Camera,

    compule_pipeline: wgpu::ComputePipeline,
    output_buffer: Buffer,
    cs_const_input_binding_group: BindGroup,
    sc_output_binding_groups: Vec<BindGroup>,
    output_copy_buffer: Buffer,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    fs_texture_bindings: Vec<BindGroup>,
    fs_sampler_binding: BindGroup,

    fps_counter: fps_counter::FPSCounter,
    frame_counter: usize,

    output_q_sender: futures_intrusive::channel::shared::GenericSender<
        parking_lot::RawMutex,
        Vec<u8>,
        futures_intrusive::buffer::GrowingHeapBuf<Vec<u8>>,
    >,
}

impl State {
    fn create_buffer<T: Sized + bytemuck::Pod>(
        device: &wgpu::Device,
        data: &[T],
        name: &str,
    ) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(name),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        })
    }

    fn create_output_buffered_binding(
        device: &wgpu::Device,
        binding_group_layout: &wgpu::BindGroupLayout,
        texture: &Texture,
        name: &str,
    ) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: binding_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture.view),
            }],
        })
    }

    fn create_compute_stage(
        device: &wgpu::Device,
        output_size: winit::dpi::PhysicalSize<u32>,
        camera_texture: &Texture,
        display_textures: &[Texture],
    ) -> (
        wgpu::ComputePipeline,
        Buffer,
        BindGroup,
        Vec<BindGroup>,
        Buffer,
    ) {
        // Буффер для матрицы дизеринга
        let dithering_matrix_buffer =
            Self::create_buffer(&device, &DITHERING_MATRIX, "Output data");

        // Пустой буффер для входных данных
        let output_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output data"),
            size: (output_size.height * output_size.width / 8) as u64,
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
            contents: bytemuck::cast_slice(&[crate::compute_config::ComputeConfig {
                width: output_size.width,
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
        let cs_module =
            device.create_shader_module(wgpu::include_wgsl!("shaders/compute_shader.wgsl"));

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
                    // Матрица дизеринга
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Выходные данные
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
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
                        binding: 4,
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
                            access: wgpu::StorageTextureAccess::WriteOnly,
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
                // Матрица дизеринга
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &dithering_matrix_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(dithering_matrix_buffer.size()),
                    }),
                },
                // Выходные данные
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &output_data_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(output_data_buffer.size()),
                    }),
                },
                // Конфигурация
                wgpu::BindGroupEntry {
                    binding: 4,
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
                Self::create_output_buffered_binding(
                    &device,
                    &cs_output_buffered_binding_layout,
                    t,
                    &format!("Display texture {}", i),
                )
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

    fn create_render_stage(
        device: &wgpu::Device,
        display_textures: &[Texture],
        out_format: wgpu::TextureFormat,
    ) -> (
        wgpu::RenderPipeline,
        Buffer,
        Buffer,
        Vec<BindGroup>,
        BindGroup,
    ) {
        // Создание буфера вершин
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "draw surface".into(),
            contents: bytemuck::cast_slice(crate::verticies::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Создание буфера индексов
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Index Buffer".into(),
            contents: bytemuck::cast_slice(crate::verticies::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Создание сэмплера для выводимой текстуры
        let display_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Display sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Создание лэйаута группы байндингов, содержащую биндинг к выходной текстуре
        let fs_texture_binding_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Fragment shader binding layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        // Создание лэйаута группы байндингов, содержащую биндинг к сэмплеру
        let fs_sampler_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Fragment shader sampler layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            }],
        });

        // Загрузка биндингов к выходной текстуре
        let fs_texture_bindings = display_textures
            .iter()
            .enumerate()
            .map(|(i, texture)| {
                let name = format!("Fragment shader input binding {i}");
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(name.as_str()),
                    layout: &fs_texture_binding_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    }],
                })
            })
            .collect::<Vec<_>>();

        // Создание группы байндингов, содержащую биндинг к семплеру вы
        let fs_sampler_binding = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Fragment shader sampler binding"),
            layout: &fs_sampler_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&display_sampler),
            }],
        });

        // Загрузка шейдеров
        let shader_module = device.create_shader_module(wgpu::include_wgsl!("shaders/shader.wgsl"));

        // Создание лэйаута графического пайплайна
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Pipeline layout"),
                bind_group_layouts: &[&fs_texture_binding_layout, &fs_sampler_layout],
                push_constant_ranges: &[],
            });

        // Создание графического пайплайна
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout), // наш layout
            vertex: wgpu::VertexState {
                // шаг 1 - вершинный шейдер
                module: &shader_module, // единица компиляции шейдера в которой лежит вызываемая точка входа
                entry_point: "vs_main", // название точки входа
                buffers: &[crate::verticies::MyVertex::desc()], // дескрипторы параметоров которые мы отправим в вершинный шейдер, слоты 0, 1...
            },
            fragment: Some(wgpu::FragmentState {
                // шаг 2 - фрагментный шейдер
                module: &shader_module, // единица компиляции шейдера в которой лежит вызываемая точка входа
                entry_point: "fs_main", // название точки входа
                targets: &[Some(wgpu::ColorTargetState {
                    format: out_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip, // каждые 3 вершины образуют треугольник, с персечениями
                strip_index_format: Some(wgpu::IndexFormat::Uint16), // тип данных для индекса вершин
                front_face: wgpu::FrontFace::Ccw, // лицевая сторона трецгольнка - это та сторона которая образована порядком следования вершин против часовой стрелки
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // отключаем тест глубины
            multisample: wgpu::MultisampleState {
                count: 1,                         // количество семплов для мультисемплинга
                mask: !0,                         // маска мультисемплинга
                alpha_to_coverage_enabled: false, // выключаем антиалиасинг
            },
            multiview: None, // это нужно для нендеринга во множкство мест одновременно
        });

        (
            render_pipeline,
            vertex_buffer,
            index_buffer,
            fs_texture_bindings,
            fs_sampler_binding,
        )
    }

    pub(crate) async fn new(
        window: Window,
        camera: nokhwa::Camera,
        output_size: PhysicalSize<u32>,
        output_q_sender: futures_intrusive::channel::shared::GenericSender<
            parking_lot::RawMutex,
            Vec<u8>,
            futures_intrusive::buffer::GrowingHeapBuf<Vec<u8>>,
        >,
    ) -> Self {
        let window_size = window.inner_size();
        let camera_size = camera.resolution();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::BUFFER_BINDING_ARRAY
                        | wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY,
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let format = surface.get_capabilities(&adapter).formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![format],
        };
        surface.configure(&device, &config);

        //--------------------------------------------------------------------------------

        // Пустая текстура для входных данных с камеры
        let camera_texture = Texture::empty(
            &device,
            (camera_size.width(), camera_size.height()),
            wgpu::TextureFormat::R8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            "Camera texture",
        );

        // Пустые текстуры для результата работы вычислительного шейдера (двойная буферизация)
        let display_textures = (0..2)
            .map(|i| {
                Texture::empty(
                    &device,
                    (output_size.width, output_size.height),
                    wgpu::TextureFormat::Rgba8Unorm,
                    wgpu::TextureUsages::TEXTURE_BINDING // Чтобы в шейдере можно было читать семплером
                        | wgpu::TextureUsages::STORAGE_BINDING, // Чтобы в шейдере можно было писать напрямую
                    &format!("Display texture {i}"),
                )
            })
            .collect::<Vec<_>>();

        //--------------------------------------------------------------------------------

        let compute_stage =
            Self::create_compute_stage(&device, output_size, &camera_texture, &display_textures);

        //--------------------------------------------------------------------------------

        let render_stage = Self::create_render_stage(&device, &display_textures, format);

        //--------------------------------------------------------------------------------

        Self {
            surface,
            device,
            queue,
            config,
            window_size,
            window,
            output_size,

            camera_texture,
            camera,

            compule_pipeline: compute_stage.0,
            output_buffer: compute_stage.1,
            cs_const_input_binding_group: compute_stage.2,
            sc_output_binding_groups: compute_stage.3,
            output_copy_buffer: compute_stage.4,

            render_pipeline: render_stage.0,
            vertex_buffer: render_stage.1,
            index_buffer: render_stage.2,
            fs_texture_bindings: render_stage.3,
            fs_sampler_binding: render_stage.4,

            fps_counter: fps_counter::FPSCounter::default(),
            frame_counter: 0,

            output_q_sender,
        }
    }

    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.window_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    pub(crate) fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub(crate) fn update(&mut self) {
        let fps = self.fps_counter.tick();
        println!("FPS: {}", fps);
    }

    pub(crate) async fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if let Ok(frame) = self.camera.frame() {
            // frame - YUV frame
            let frame = frame.decode_image::<pixel_format::LumaFormat>().unwrap();

            // load data from diffuse_rgba to diffuse_texture allocated in GPU memory above
            let dimensions = frame.dimensions();
            self.queue.write_texture(
                // Куда копировать данные
                wgpu::ImageCopyTexture {
                    aspect: wgpu::TextureAspect::All,
                    texture: &self.camera_texture.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                // Источник данных
                frame.as_raw(),
                // Как копировать данные, преобразования форматов, например
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: std::num::NonZeroU32::new(dimensions.0),
                    rows_per_image: std::num::NonZeroU32::new(dimensions.1),
                },
                self.camera_texture.texture.size(), // размер текстуры
            );
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Compute stage
        {
            // Вычислительный шейдер
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
            });

            // Устанавливаем созданный ранее пайплайн для вычислений
            compute_pass.set_pipeline(&self.compule_pipeline);

            // Устанавливаем в слот 0 группу привязок для входных данных
            compute_pass.set_bind_group(0, &self.cs_const_input_binding_group, &[]);

            // Устанавливаем в слот 1 группу привязок для выходной текстуры, номер зависит от четности кадра
            compute_pass.set_bind_group(
                1,
                &self.sc_output_binding_groups[self.frame_counter % 2],
                &[],
            );

            // Выполняем вычисления (для каждого пикселя выходной текстуры)
            compute_pass.dispatch_workgroups(
                (self.output_size.width * self.output_size.height)
                    / (core::mem::size_of::<u32>() * 8) as u32,
                1,
                1,
            );
        }

        // Render stage
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            // Устанавливаем созданный ранее пайплайн для рендеринга
            render_pass.set_pipeline(&self.render_pipeline);
            // Передаем группу привязок 0 в шейдер, зависит от нечетности кадра
            render_pass.set_bind_group(
                0,
                &self.fs_texture_bindings[((self.frame_counter + 1) % 2)],
                &[],
            );
            // Передаем группу привязок 1 в шейдер (семплер)
            render_pass.set_bind_group(1, &self.fs_sampler_binding, &[]);
            // передаем буфер с вершинами в слот 0 шейдера
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // передаем буфер с индексами в слот 1 шейдера
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // нарисовать все вершины по индексу начиная с 0-ого 1 раз
            render_pass.draw_indexed(0..(crate::verticies::INDICES.len() as u32), 0, 0..1);
        }

        //---------------------------------------------------------------------

        // Нарпямую читать из STORAGE буфера нельзя, нужно скопировать результат в отдельный буфер который поддерживает mapping
        encoder.copy_buffer_to_buffer(
            &self.output_buffer,
            0,
            &self.output_copy_buffer,
            0,
            self.output_buffer.size(),
        );

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        //---------------------------------------------------------------------

        // data stage
        {
            self.output_copy_buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |_| {});

            // Poll the device in a blocking manner so that our future resolves.
            // In an actual application, `device.poll(...)` should
            // be called in an event loop or on another thread.
            self.device.poll(wgpu::Maintain::Wait);

            // Прочитать данные из выходного буфера
            let out_buf_view = self.output_copy_buffer.slice(..).get_mapped_range();

            // Преобразовать данные в вектор байтов
            let out_buf = unsafe {
                std::slice::from_raw_parts(out_buf_view.as_ptr() as *const u8, out_buf_view.len())
            }
            .to_vec();

            let _ = self.output_q_sender.try_send(out_buf);
        }
        // unmap buffer
        self.output_copy_buffer.unmap();

        // Переключаемся на следующий кадр
        self.frame_counter = self.frame_counter.wrapping_add(1);

        Ok(())
    }
}
