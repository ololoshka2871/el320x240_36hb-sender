use core::default::Default;
use std::iter;

use nokhwa::pixel_format;
use wgpu::{util::DeviceExt, BindGroup, Buffer};
use winit::{event::*, window::Window};

pub(crate) struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate) size: winit::dpi::PhysicalSize<u32>,
    window: Window,

    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    diffuse_bind_group: BindGroup,
    _diffuse_texture: crate::texture::Texture,
    camera_texture: crate::texture::Texture,

    /*
    cs_pipeline: wgpu::ComputePipeline,
    cs_bind_group: BindGroup,
    _staging_buffer: Buffer,
    _storage_buffer: Buffer,

    numbers: Vec<u32>,
    */
    camera: nokhwa::Camera,

    fps_counter: fps_counter::FPSCounter,
}

impl State {
    pub(crate) async fn new(window: Window, camera: nokhwa::Camera) -> Self {
        let size = window.inner_size();

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
                    features: wgpu::Features::empty(),
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
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![format],
        };
        surface.configure(&device, &config);

        //--------------------------------------------------------------------------------

        // Встроить данные картинки в бинарник, и получить на них ссылку
        let diffuse_bytes = include_bytes!("happy-tree.png");
        // Создаем текстуру из данных картинки
        let diffuse_texture =
            crate::texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png")
                .unwrap();

        let camera_texture = crate::texture::Texture::empty(
            &device,
            (320, 240),
            wgpu::TextureFormat::R8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            "Camera frame texture",
        );

        // Создаем группу биндингов для текстуры
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // Загружаем информацию о текстуре в группу биндингов в те же слоты что объявлены в texture_bind_group_layout
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&camera_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&camera_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        //--------------------------------------------------------------------------------

        // load and complie shader from file (see shaders/shader.wgsl)
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/shader.wgsl"));

        /*
        // compule shader from file (see shaders/compute_shader.wgsl)
        let cs_module =
            device.create_shader_module(wgpu::include_wgsl!("shaders/compute_shader.wgsl"));
            */

        // create render pipeline layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout], // здесь описываются слоты для биндингов 0, 1, 2...
                push_constant_ranges: &[],
            });

        // create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout), // наш layout
            vertex: wgpu::VertexState {
                // шаг 1 - вершинный шейдер
                module: &shader, // единица компиляции шейдера в которой лежит вызываемая точка входа
                entry_point: "vs_main", // название точки входа
                buffers: &[crate::verticies::MyVertex::desc()], // дескрипторы параметоров которые мы отправим в вершинный шейдер, слоты 0, 1...
            },
            fragment: Some(wgpu::FragmentState {
                // шаг 2 - фрагментный шейдер
                module: &shader, // единица компиляции шейдера в которой лежит вызываемая точка входа
                entry_point: "fs_main", // название точки входа
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "draw surface".into(),
            contents: bytemuck::cast_slice(crate::verticies::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: "Index Buffer".into(),
            contents: bytemuck::cast_slice(crate::verticies::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        /*
        // test data
        let numbers = vec![1, 2, 3, 4];

        // Gets the size in bytes of the buffer.
        let slice_size = numbers.len() * std::mem::size_of::<u32>();
        let test_data_size = slice_size as wgpu::BufferAddress;

        // Create the buffer for input data for compute shader
        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: test_data_size,
            usage: wgpu::BufferUsages::MAP_READ  //allows it to be read (outside the shader).
                | wgpu::BufferUsages::COPY_DST, // allows it to be the destination of the copy.
            mapped_at_creation: false,
        });

        // Create the buffer for output data from compute shader
        let storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Storage Buffer"),
            contents: bytemuck::cast_slice(&numbers),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        // Instantiates the compute pipeline.
        let cs_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: None,
            module: &cs_module,
            entry_point: "main",
        });

        // Instantiates the bind group
        let cs_bind_group_layout = cs_pipeline.get_bind_group_layout(0);
        let cs_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &cs_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: storage_buffer.as_entire_binding(),
            }],
        });
        */

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            diffuse_bind_group,
            _diffuse_texture: diffuse_texture,
            camera_texture,

            /*
            cs_pipeline,
            cs_bind_group,
            _staging_buffer: staging_buffer,
            _storage_buffer: storage_buffer,

            numbers,
            */
            camera,

            fps_counter: fps_counter::FPSCounter::default(),
        }
    }

    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
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

    pub(crate) fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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
            // Передаем группу биндов в слот 0 шейдера
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            // передаем буфер с вершинами в слот 0 шейдера
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // передаем буфер с индексами в слот 1 шейдера
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // нарисовать все вершины по индексу начиная с 0-ого 1 раз
            render_pass.draw_indexed(0..(crate::verticies::INDICES.len() as u32), 0, 0..1);
        }

        /*
        {
            // Compute shader
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
            });
            cpass.set_pipeline(&self.cs_pipeline); // Устанавливаем созданный ранее пайплайн для вычислений
            cpass.set_bind_group(0, &self.cs_bind_group, &[]); // Передаем группу биндов в слот 0 вычислитльного шейдера
            cpass.dispatch_workgroups(self.numbers.len() as u32, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed

            // https://github.com/gfx-rs/wgpu/blob/master/wgpu/examples/hello-compute/main.rs
            // Sets adds copy operation to command encoder.
            // Will copy data from storage buffer on GPU to staging buffer on CPU.
            encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);
        }
        */

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
