/*
 * Copyright 2021 l1npengtul <l1npengtul@protonmail.com> / The Nokhwa Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

// Some assembly required. For developers 7 and up.

use bytes::{BufMut, BytesMut};
use flume::{Receiver, Sender};
use glium::{
    implement_vertex, index::PrimitiveType, program, texture::RawImage2d, uniform, Display,
    IndexBuffer, Surface, Texture2d, VertexBuffer,
};
use glutin::{event_loop::EventLoop, window::WindowBuilder, ContextBuilder};
use nokhwa::{
    pixel_format::LumaFormat,
    query,
    utils::{
        ApiBackend, CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType,
        Resolution,
    },
    Camera,
};
use std::{process::exit, time::Duration};
use structopt::StructOpt;

#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

fn capturer(
    camera_indes: u32,
    channel: Sender<Vec<u8>>,
    w: u32,
    h: u32,
    black: i16,
    white: i16,
    fps: u32,
    dithering: bool,
) {
    let format = RequestedFormat::new::<LumaFormat>(RequestedFormatType::Closest(
        CameraFormat::new(Resolution::new(w, h), FrameFormat::YUYV, fps),
    ));

    let mut camera = Camera::new(CameraIndex::Index(camera_indes), format).unwrap();

    // open stream
    camera.open_stream().unwrap();
    loop {
        if let Ok(frame) = camera.frame() {
            // frame - RGB frame
            let frame = frame.decode_image::<LumaFormat>().unwrap();

            // convert frame to grayscale buffer of pixels
            let grayscale_frame = frame
                .enumerate_pixels()
                .map(|(_x, _y, pixel)| pixel.0[0])
                .collect::<Vec<_>>();
            let df = if dithering {
                let matrix = [[-1, 3], [3, 2i16]];
                image_dithering(
                    &grayscale_frame,
                    matrix,
                    w as usize,
                    h as usize,
                    black,
                    white,
                )
            } else {
                grayscale_frame
                    .into_iter()
                    .map(|x| {
                        if x as i16 > black {
                            [0xff, 0xff, 0xff]
                        } else {
                            [0x00, 0x00, 0x00]
                        }
                    })
                    .flatten()
                    .collect::<Vec<_>>()
            };

            let _send = channel.send(df);
        }
    }
}

fn image_dithering<const M: usize, const N: usize>(
    gray_pixels: &[u8],
    dithering_matrix: [[i16; M]; N],
    w: usize,
    h: usize,

    black: i16,
    white: i16,
) -> Vec<u8> {
    fn iterate_diffusion_matrix<const M: usize, const N: usize>(
        xres: usize,
        yres: usize,
        x: usize,
        y: usize,
        convert_buf: &mut [i16],
        pixel: i16,
        error: i16,
        dithering_matrix: [[i16; M]; N],

        black: i16,
        white: i16,
    ) {
        for i in 0..M {
            /* diffusion matrix column */
            for j in 0..N {
                /* skip pixels out of zone */
                if (x + i >= xres) || (y + j >= yres) {
                    continue;
                }
                let write_pos = &mut convert_buf[(y + j) * xres + x + i];
                let coeff = dithering_matrix[i][j];
                if -1 == coeff {
                    /* pixel itself */
                    *write_pos = pixel;
                } else {
                    let mut p = *write_pos + error * coeff;

                    if p > white {
                        p = white;
                    }
                    if p < black {
                        p = white;
                    }
                    *write_pos = p;
                }
            }
        }
    }

    #[allow(unused)]
    static GAMMA_CORRECTION_TABLE: [i16; 256] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2,
        2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10,
        11, 11, 11, 12, 12, 13, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21,
        22, 22, 23, 23, 24, 25, 25, 26, 26, 27, 28, 28, 29, 30, 30, 31, 32, 33, 33, 34, 35, 35, 36,
        37, 38, 39, 39, 40, 41, 42, 43, 43, 44, 45, 46, 47, 48, 49, 49, 50, 51, 52, 53, 54, 55, 56,
        57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 73, 74, 75, 76, 77, 78, 79, 81,
        82, 83, 84, 85, 87, 88, 89, 90, 91, 93, 94, 95, 97, 98, 99, 100, 102, 103, 105, 106, 107,
        109, 110, 111, 113, 114, 116, 117, 119, 120, 121, 123, 124, 126, 127, 129, 130, 132, 133,
        135, 137, 138, 140, 141, 143, 145, 146, 148, 149, 151, 153, 154, 156, 158, 159, 161, 163,
        165, 166, 168, 170, 172, 173, 175, 177, 179, 181, 182, 184, 186, 188, 190, 192, 194, 196,
        197, 199, 201, 203, 205, 207, 209, 211, 213, 215, 217, 219, 221, 223, 225, 227, 229, 231,
        234, 236, 238, 240, 242, 244, 246, 248, 251, 253, 255,
    ];

    let mut convert_buf = Vec::<i16>::with_capacity(w * h);
    unsafe {
        convert_buf.set_len(w * h);
    }

    for x in 0..w {
        for y in 0..h {
            convert_buf[y * w + x] = GAMMA_CORRECTION_TABLE[gray_pixels[y * w + x] as usize];
        }
    }

    /* Image Dithering */
    for x in 0..w {
        for y in 0..h {
            let mut pixel = convert_buf[y * w + x];
            let error_b = pixel - black;
            let error_w = pixel - white;
            let mut error;

            /* what color close? */
            if error_b.abs() >= error_w.abs() {
                /* white */
                error = error_w;
                pixel = white;
            } else {
                /* black */
                error = error_b;
                pixel = black;
            }

            error /= 8;

            iterate_diffusion_matrix(
                w,
                h,
                x,
                y,
                &mut convert_buf,
                pixel,
                error,
                dithering_matrix,
                black,
                white,
            );
        }
    }

    convert_buf
        .into_iter()
        .map(|p| [p as u8, p as u8, p as u8])
        .flatten()
        .collect()
}

fn run_glium(recv: Receiver<Vec<u8>>, dimensions: (u32, u32), name: String) {
    let gl_event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new().with_title(name);
    let context_builder = ContextBuilder::new().with_vsync(true);
    let gl_display = Display::new(window_builder, context_builder, &gl_event_loop).unwrap();

    implement_vertex!(Vertex, position, tex_coords);

    let vert_buffer = VertexBuffer::new(
        &gl_display,
        &[
            Vertex {
                position: [-1.0, -1.0],
                tex_coords: [0.0, 0.0],
            },
            Vertex {
                position: [-1.0, 1.0],
                tex_coords: [0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0],
                tex_coords: [1.0, 1.0],
            },
            Vertex {
                position: [1.0, -1.0],
                tex_coords: [1.0, 0.0],
            },
        ],
    )
    .unwrap();

    let idx_buf =
        IndexBuffer::new(&gl_display, PrimitiveType::TriangleStrip, &[1_u16, 2, 0, 3]).unwrap();

    let program = program!(&gl_display,
        140 => {
            vertex: "
            #version 140
            uniform mat4 matrix;
            in vec2 position;
            in vec2 tex_coords;
            out vec2 v_tex_coords;
            void main() {
                gl_Position = matrix * vec4(position, 0.0, 1.0);
                v_tex_coords = tex_coords;
            }
        ",

            fragment: "
            #version 140
            uniform sampler2D tex;
            in vec2 v_tex_coords;
            out vec4 f_color;
            void main() {
                f_color = texture(tex, v_tex_coords);
            }
        "
        },
    )
    .unwrap();

    // run the event loop

    gl_event_loop.run(move |event, _window, ctrl| {
        let graysacle_frame = recv.recv().unwrap();

        let raw_data = RawImage2d::from_raw_rgb(graysacle_frame, dimensions);
        let gl_texture = Texture2d::new(&gl_display, raw_data).unwrap();

        let uniforms = uniform! {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, -1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0f32]
            ],
            tex: &gl_texture
        };

        let mut target = gl_display.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target
            .draw(
                &vert_buffer,
                &idx_buf,
                &program,
                &uniforms,
                &Default::default(),
            )
            .unwrap();
        target.finish().unwrap();

        if let glutin::event::Event::WindowEvent { event, .. } = event {
            if event == glutin::event::WindowEvent::CloseRequested {
                *ctrl = glutin::event_loop::ControlFlow::Exit;
            }
        }
    })
}

fn display_sender(port: String, rx: Receiver<Vec<u8>>, tx: Sender<Vec<u8>>, black: u8) {
    const BLOCK_SIZE: usize = 64;
    const BLOCK_SIZE_PYLOAD: usize = BLOCK_SIZE - std::mem::size_of::<u32>();

    let mut port = serialport::new(port, 15000000)
        .timeout(Duration::from_millis(5))
        .open()
        .expect("Failed to open port");

    loop {
        let frame = rx.recv().unwrap();

        let bin_data = frame
            .chunks(3 * 8) // RGB(3 bytes) * 8 pixels ->[map to]-> 1 byte
            .map(|pixels| {
                let mut res = 0u8;
                for (i, pixel) in pixels.chunks(3).enumerate() {
                    let pixel = pixel[0] as u16 + pixel[1] as u16 + pixel[2] as u16;
                    if pixel > black as u16 * 3 {
                        res |= 1 << (7 - i);
                    }
                }
                res
            })
            .collect::<Vec<_>>();

        bin_data
            .chunks(BLOCK_SIZE_PYLOAD)
            .enumerate()
            .for_each(|(i, data)| {
                let offset = i * BLOCK_SIZE_PYLOAD;
                let mut buf: BytesMut = BytesMut::new();
                // offset in bytes
                buf.put_u32_le(offset as u32);
                buf.put_slice(data);

                port.write_all(&buf).unwrap();
            });

        let _ = tx.send(frame);
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    about = "Send image from OBS virtual camera to el320x240_36hb over virtual serial port"
)]
struct Cli {
    /// camera ID
    #[structopt(short, default_value = "0")]
    id: u32,

    /// width
    #[structopt(long, default_value = "320")]
    width: u32,

    /// heigth
    #[structopt(long, default_value = "240")]
    heigth: u32,

    /// fps
    #[structopt(long, default_value = "30")]
    fps: u32,

    /// Serial port
    #[structopt(short, default_value = "/dev/ttyACM0")]
    port: String,

    /// use dithering
    #[structopt(short)]
    dither: bool,

    /// level of black color, 0-255
    #[structopt(default_value = "0")]
    black_lvl: u8,

    /// level of white color, 0-255
    #[structopt(default_value = "255")]
    white_lvl: u8,

    /// list available cameras
    #[structopt(short)]
    list: bool,
}

fn main() {
    let args = Cli::from_args();

    let width = args.width as u32;
    let height = args.heigth as u32;
    let camera_id = args.id;
    let mut black = args.black_lvl as i16;
    let white = args.white_lvl as i16;
    let fps = args.fps;
    let dithering = args.dither;

    if !dithering {
        println!("Dithering is disabled");
        if black == 0 {
            black = 0xff / 2;
        }
    }

    nokhwa::nokhwa_initialize(|_| {});

    let cameras = query(ApiBackend::Auto).unwrap();

    if args.list {
        println!("Found cameras:");
        cameras
            .iter()
            .for_each(|cam| println!("{}: {}", cam.index(), cam.human_name()));
        exit(0);
    }

    if cameras.is_empty() {
        println!("No web-cameras found!");
        exit(1);
    }

    let (capture, gip_send) = flume::unbounded();
    let (gip_sent, recv) = flume::unbounded();

    // start capture thread
    std::thread::spawn(move || {
        capturer(
            camera_id, capture, width, height, black, white, fps, dithering,
        )
    });

    std::thread::spawn(move || display_sender(args.port, gip_send, gip_sent, black as u8));

    // run glium
    run_glium(recv, (width, height), format!("Camera: {camera_id}"));
}
