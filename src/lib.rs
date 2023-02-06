mod compute_config;
mod el320x240_36hb_sender;
mod state;
mod texture;
mod verticies;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use nokhwa::utils::*;

pub async fn run() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // We need to initialize Nokhwa before we can use it
    nokhwa::nokhwa_initialize(|_| {});

    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto).unwrap();
    if cameras.is_empty() {
        println!("No web-cameras found!");
        std::process::exit(1);
    }

    // camera capture format
    let format =
        RequestedFormat::new::<nokhwa::pixel_format::LumaFormat>(RequestedFormatType::Closest(
            CameraFormat::new(Resolution::new(640, 480), FrameFormat::YUYV, 30),
        ));

    // open camera
    let mut camera = nokhwa::Camera::new(CameraIndex::Index(0), format).unwrap();
    camera.open_stream().unwrap();

    // channel to get processed data from GPU
    let (sender, receiver) = futures_intrusive::channel::shared::channel::<Vec<u8>>(1);

    // start sender thread
    std::thread::spawn(move || {
        el320x240_36hb_sender::display_sender("/dev/ttyACM1".to_string(), receiver);
    });

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = state::State::new(
        window,
        camera,
        winit::dpi::PhysicalSize::new(320, 240),
        sender,
    )
    .await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    // UPDATED!
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &&mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match pollster::block_on(state.render()) {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.window_size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                    Err(wgpu::SurfaceError::Timeout) => println!("Surface timeout"),
                };
            }
            Event::RedrawEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                state.window().request_redraw();
            }
            _ => {}
        }
    });
}
