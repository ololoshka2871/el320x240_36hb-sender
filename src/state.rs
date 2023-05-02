use core::default::Default;
use std::iter;

use nokhwa::pixel_format;
use wgpu::{util::DeviceExt, BindGroup, Buffer};
use winit::{dpi::PhysicalSize, event::*, window::Window};

use crate::texture::Texture;

pub(crate) struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub(crate) window_size: winit::dpi::PhysicalSize<u32>,
    window: Window,

    camera_texture: crate::texture::Texture,
    camera: nokhwa::Camera,

    compule_pipeline: wgpu::ComputePipeline,
    output_buffer: Buffer,
    cs_const_input_binding_group: BindGroup,
    sc_output_binding_groups: Vec<BindGroup>,
    output_copy_buffer: Buffer,
    compute_stage: Box<dyn crate::compute_stage_control::ComputeStageControl>,

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
    /*
    fn load_cs_shader(
        device: &wgpu::Device,
        algo: crate::args::DitherAlgorithm,
    ) -> wgpu::ShaderModule {
        match algo {
            crate::args::DitherAlgorithm::Threshold => device
                .create_shader_module(wgpu::include_wgsl!("shaders/compute_shader_threshold.wgsl")),
            crate::args::DitherAlgorithm::Ordered => device
                .create_shader_module(wgpu::include_wgsl!("shaders/compute_shader_ordered.wgsl")),
            crate::args::DitherAlgorithm::Pinwheel => device
                .create_shader_module(wgpu::include_wgsl!("shaders/compute_shader_pinwheel.wgsl")),
        }
    }
    */

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
        let shader_module =
            device.create_shader_module(wgpu::include_wgsl!("shaders/graphic_shader.wgsl"));

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
        algo: crate::args::DitherAlgorithm,
        lvls: (f32, f32),
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
                        | wgpu::Features::STORAGE_RESOURCE_BINDING_ARRAY
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
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

        let compute_stage = crate::compute_stage_control::create_compute_stage(algo, output_size);

        let (
            compule_pipeline,
            output_buffer,
            cs_const_input_binding_group,
            sc_output_binding_groups,
            output_copy_buffer,
        ) = compute_stage.configure(&device, &camera_texture, &display_textures, lvls);

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

            camera_texture,
            camera,

            compule_pipeline,
            output_buffer,
            cs_const_input_binding_group,
            sc_output_binding_groups,
            output_copy_buffer,
            compute_stage,

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

            // Выполняем вычисления
            self.compute_stage.call_dispatch(&mut compute_pass);
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
            // Заказ маппинга буфера
            self.output_copy_buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |_| { /* empty */ });

            // Poll the device in a blocking manner so that our future resolves.
            // In an actual application, `device.poll(...)` should
            // be called in an event loop or on another thread.
            self.device.poll(wgpu::Maintain::Wait);

            // Окно доступа к буферу
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
