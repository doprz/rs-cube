// Copyright (c) 2023 doprz
// SPDX-License-Identifier: MIT OR Apache-2.0
use clap::Parser;
use libc::{ioctl, signal, winsize, SIGINT, STDOUT_FILENO, TIOCGWINSZ};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
pub mod ansi_escape_code;
pub mod color;
pub mod init;

static SIGINT_CALLED: AtomicBool = AtomicBool::new(false);

const CUBE_SIZE: f32 = 1.0;
const GRID_LINE_COLOR: &str = color::BLACK;
const K2: f32 = 10.0;

enum Axes {
    AFront,
    ABack,
    BFront,
    BBack,
    CFront,
    CBack,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Enable debug info
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[arg(short, long, default_value_t = 60u32)]
    fps_limit: u32,
}

#[derive(Clone, Copy)]
struct Vector3f {
    x: f32,
    y: f32,
    z: f32,
}

impl Vector3f {
    fn mag(&self) -> f32 {
        let x: f32 = self.x;
        let y: f32 = self.y;
        let z: f32 = self.z;

        (x * x + y * y + z * z).sqrt()
    }

    fn norm(&mut self) {
        let x: f32 = self.x;
        let y: f32 = self.y;
        let z: f32 = self.z;

        let mag: f32 = self.mag();
        let oomag: f32 = 1.0 / mag;

        if mag > 0.0 {
            self.x = x * oomag;
            self.y = y * oomag;
            self.z = z * oomag;
        }
    }

    fn dot(&self, vec: &Vector3f) -> f32 {
        self.x * vec.x + self.y * vec.y + self.z * vec.z
    }

    fn rot(&mut self, trig_values: &[f32]) {
        let trig_values = &trig_values[..6];
        let (sin_a, sin_b, sin_c) = (&trig_values[0], &trig_values[2], &trig_values[4]);
        let (cos_a, cos_b, cos_c) = (&trig_values[1], &trig_values[3], &trig_values[5]);

        let x_arg1 = cos_a * sin_b * sin_c - sin_a * cos_c;
        let x_arg2 = cos_a * sin_b * cos_c + sin_a * sin_c;
        let y_arg1 = sin_a * sin_b * sin_c + cos_a * cos_c;
        let y_arg2 = sin_a * sin_b * cos_c - cos_a * sin_c;

        let prev = self.clone();

        self.x = cos_a * cos_b * prev.x + (x_arg1) * prev.y + (x_arg2) * prev.z;
        self.y = sin_a * cos_b * prev.x + (y_arg1) * prev.y + (y_arg2) * prev.z;
        self.z = -prev.x * sin_b + prev.y * cos_b * sin_c + prev.z * cos_b * cos_c;
    }
}

struct AxesLuminance {
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
}

fn update_buffers(
    i: f32,
    j: f32,
    k: f32,
    width: u16,
    height: u16,
    buffer: &mut [char],
    zbuffer: &mut [f32],
    cbuffer: &mut [&str],
    trig_values: &[f32],
    char_color: bool,
    axes: Axes,
    luminance: f32,
) {
    assert!(zbuffer.len() == buffer.len() && cbuffer.len() == buffer.len() && luminance <= 1.0);
    let trig_values = &trig_values[..6];

    let (sin_a, sin_b, sin_c) = (trig_values[0], trig_values[2], trig_values[4]);
    let (cos_a, cos_b, cos_c) = (trig_values[1], trig_values[3], trig_values[5]);

    let cos_a_sin_b = cos_a * sin_b;
    let sin_a_sin_b = sin_a * sin_b;

    let x: f32 = cos_a * cos_b * j
        + (cos_a_sin_b * sin_c - sin_a * cos_c) * i
        + (cos_a_sin_b * cos_c + sin_a * sin_c) * k;
    let y: f32 = sin_a * cos_b * j
        + (sin_a_sin_b * sin_c + cos_a * cos_c) * i
        + (sin_a_sin_b * cos_c - cos_a * sin_c) * k;
    let z: f32 = -j * sin_b + i * cos_b * sin_c + k * cos_b * cos_c + K2;

    let ooz: f32 = 1.0 / z; // "One over z"

    let k1: f32 = ((width as f32) * K2 * 3.0) / (8.0 * ((3.0_f32).sqrt() * CUBE_SIZE));
    let xp: u32 = (((width as f32) / (2.0)) + (k1 * ooz * x)) as u32;
    let yp: u32 = (((height as f32) / (2.0)) - (k1 * ooz * y)) as u32;

    let index: usize = (xp + yp * width as u32).try_into().unwrap();
    let index_limit: usize = buffer.len();
    let luminance_index: usize = (luminance * 11.0) as usize;

    // Luminance ranges from -1 to +1 for the dot product of the plane normal and light source normalized 3D unit vectors
    // If the luminance > 0, then the plane is facing towards the light source
    // else if luminance < 0, then the plane is facing away from the light source
    // else if luminance = 0, then the plane and the light source are perpendicular
    if index < index_limit && ooz > zbuffer[index] {
        zbuffer[index] = ooz;
        cbuffer[index] = if char_color {
            match axes {
                Axes::AFront => color::YELLOW,
                Axes::ABack => color::WHITE,
                Axes::BFront => color::GREEN,
                Axes::BBack => color::BLUE,
                Axes::CFront => color::BOLD_RED,
                Axes::CBack => color::RED,
            }
        } else {
            GRID_LINE_COLOR
        };
        buffer[index] =
            ".,-~:;=!*#$@".as_bytes()[if luminance > 0.0 { luminance_index } else { 0 }] as char;
    }
}

fn get_axes_luminance(rotated_light_source: &Vector3f, trig_values: &[f32]) -> AxesLuminance {
    // Axis A
    let mut a_surface_normal_front = Vector3f {
        x: 0.0,
        y: 0.0,
        z: CUBE_SIZE,
    };
    let mut a_surface_normal_back = Vector3f {
        x: 0.0,
        y: 0.0,
        z: -CUBE_SIZE,
    };

    a_surface_normal_front.rot(trig_values);
    a_surface_normal_back.rot(trig_values);
    a_surface_normal_front.norm();
    a_surface_normal_back.norm();

    // Axis B
    let mut b_surface_normal_front = Vector3f {
        x: 0.0,
        y: CUBE_SIZE,
        z: 0.0,
    };
    let mut b_surface_normal_back = Vector3f {
        x: 0.0,
        y: -CUBE_SIZE,
        z: 0.0,
    };

    b_surface_normal_front.rot(trig_values);
    b_surface_normal_back.rot(trig_values);
    b_surface_normal_front.norm();
    b_surface_normal_back.norm();

    // Axis C
    let mut c_surface_normal_front = Vector3f {
        x: CUBE_SIZE,
        y: 0.0,
        z: 0.0,
    };
    let mut c_surface_normal_back = Vector3f {
        x: -CUBE_SIZE,
        y: 0.0,
        z: 0.0,
    };

    c_surface_normal_front.rot(trig_values);
    c_surface_normal_back.rot(trig_values);
    c_surface_normal_front.norm();
    c_surface_normal_back.norm();

    AxesLuminance {
        a: (
            a_surface_normal_front.dot(rotated_light_source),
            a_surface_normal_back.dot(rotated_light_source),
        ),
        b: (
            b_surface_normal_front.dot(rotated_light_source),
            b_surface_normal_back.dot(rotated_light_source),
        ),
        c: (
            c_surface_normal_front.dot(rotated_light_source),
            c_surface_normal_back.dot(rotated_light_source),
        ),
    }
}

fn render_frame<'a>(
    mut handle: impl Write,
    width: u16,
    height: u16,
    points: &[init::Point3D],
    points_color: &[bool],
    points_axis_range: &init::PointsAxisRange,
    buffer: &mut [char],
    buffer_prev: &mut [char],
    cbuffer: &mut [&'a str],
    cbuffer_prev: &mut [&'a str],
    zbuffer: &mut [f32],
    trig_values: &[f32],
    rotated_light_source: &Vector3f,
) {
    buffer_prev.copy_from_slice(buffer);
    cbuffer_prev.copy_from_slice(cbuffer);

    buffer.fill(' ');
    cbuffer.fill(color::RESET);
    zbuffer.fill(0.0);

    let points_color = &points_color[..points.len()];
    let axes_luminance = get_axes_luminance(rotated_light_source, trig_values);

    for index in 0..points.len() {
        let point = &points[index];
        let color = points_color[index];
        let (axes, luminance) = if index < points_axis_range.a {
            if index & 1 == 0 {
                (Axes::AFront, axes_luminance.a.0)
            } else {
                (Axes::ABack, axes_luminance.a.1)
            }
        } else if index < points_axis_range.b {
            if index & 1 == 0 {
                (Axes::BFront, axes_luminance.b.0)
            } else {
                (Axes::BBack, axes_luminance.b.1)
            }
        } else {
            if index & 1 == 0 {
                (Axes::CFront, axes_luminance.c.0)
            } else {
                (Axes::CBack, axes_luminance.c.1)
            }
        };
        update_buffers(
            point.x,
            point.y,
            point.z,
            width,
            height,
            buffer,
            zbuffer,
            cbuffer,
            trig_values,
            color,
            axes,
            luminance,
        );
    }

    let l_cbuffer = &cbuffer[..buffer.len()];
    let l_buffer_prev = &buffer_prev[..buffer.len()];
    let l_cbuffer_prev = &cbuffer_prev[..buffer.len()];
    let mut prev_set_color: &str = color::RESET;

    for index in 0..buffer.len() {
        let val = buffer[index];
        let color = l_cbuffer[index];
        if (val == l_buffer_prev[index]) && (color == l_cbuffer_prev[index]) {
            continue;
        }

        let x: u16 = (index % width as usize).try_into().unwrap();
        let y: u16 = (index / width as usize).try_into().unwrap();

        // Move cursor, add color, and print char
        if color == prev_set_color {
            write!(
                handle,
                "{}{}",
                ansi_escape_code::SetCursorPos(y + 1, x + 1),
                val
            )
            .unwrap();
        } else {
            write!(
                handle,
                "{}{}{}",
                ansi_escape_code::SetCursorPos(y + 1, x + 1),
                color,
                val
            )
            .unwrap();
            prev_set_color = color;
        }
    }
}

fn get_term_size() -> Result<(u16, u16), &'static str> {
    let mut size = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    unsafe {
        ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size);
        if size.ws_col != 0 && size.ws_row != 0 {
            Ok((size.ws_col, size.ws_row))
        } else {
            Err("ioctl failed()")
        }
    }
}

fn handle_exit() {
    print!("{}", ansi_escape_code::EraseScreen);
    print!("{}", ansi_escape_code::DisableAltBuffer);
    print!("{}", color::RESET);
    print!("{}", ansi_escape_code::CursorVisible);
}

fn handle_sigint() {
    SIGINT_CALLED.store(true, Ordering::Relaxed);
}

fn main() {
    unsafe {
        signal(SIGINT, handle_sigint as usize);
    }

    let args = Args::parse();

    print!("{}", ansi_escape_code::EnableAltBuffer);
    print!("{}", ansi_escape_code::EraseScreen);
    print!("{}", ansi_escape_code::CursorInvisible);

    let mut width: u16 = 100;
    let mut height: u16 = 50;

    let term_size = get_term_size();

    match term_size {
        Ok(s) => {
            width = s.0;
            height = s.1;
        }
        Err(err) => {
            println!("{}", err)
        }
    }

    let stdout = io::stdout(); // get the global stdout entity
                               // wrap that handle in a buffer and aquire a lock on it
    let mut handle = io::BufWriter::with_capacity((width * height * 3).into(), stdout.lock());

    let mut buffer: Vec<char> = vec![' '; (width * height).into()];
    let mut buffer_prev: Vec<char> = vec![' '; (width * height).into()];

    let mut cbuffer: Vec<&str> = vec![color::RESET; (width * height).into()];
    let mut cbuffer_prev: Vec<&str> = vec![color::RESET; (width * height).into()];

    let mut zbuffer: Vec<f32> = vec![0.0; (width * height).into()];

    assert!(
        zbuffer.len() == buffer.len()
            && cbuffer.len() == buffer.len()
            && buffer_prev.len() == buffer.len()
            && cbuffer_prev.len() == buffer.len()
    );

    let spacing: f32 = 3.0 / width as f32;
    let _k1: f32 = ((width as f32) * K2 * 3.0) / (8.0 * (3_f32.sqrt() * CUBE_SIZE));

    let points_size = ((CUBE_SIZE * CUBE_SIZE) / spacing).round() as usize;
    let mut points: Vec<init::Point3D> = Vec::with_capacity(points_size);
    let mut points_color: Vec<bool> = Vec::with_capacity(points_size);
    let mut points_axis_range = init::PointsAxisRange { a: 0, b: 0, c: 0 };

    let light_source = Vector3f {
        x: 0.0,
        y: 1.0,
        z: -1.0,
    };
    let mut rotated_light_source = Vector3f {
        x: 0.0,
        y: 1.0,
        z: -1.0,
    };

    let total_frames = 10_000;
    let mut frame_times: Vec<u128> = Vec::with_capacity(total_frames);

    let mut a: f32 = -std::f32::consts::FRAC_PI_2; // Axis facing the screen (z-axis)
    let mut b: f32 = -std::f32::consts::FRAC_PI_2; // Up / Down axis (y-axis)
    let mut c: f32 = std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4; // Left / Right axis (x-axis)

    // Rotated Light Source
    let d: f32 = 0.0;
    let e: f32 = 0.0;
    let f: f32 = 0.0;

    let sin_d: f32 = d.sin();
    let cos_d: f32 = d.cos();

    let sin_e: f32 = e.sin();
    let cos_e: f32 = e.cos();

    let sin_f: f32 = f.sin();
    let cos_f: f32 = f.cos();

    rotated_light_source.x = cos_d * cos_e * light_source.x
        + (cos_d * sin_e * sin_f - sin_d * cos_f) * light_source.y
        + (cos_d * sin_e * cos_f + sin_d * sin_f) * light_source.z;
    rotated_light_source.y = sin_d * cos_e * light_source.x
        + (sin_d * sin_e * sin_f + cos_d * cos_f) * light_source.y
        + (sin_d * sin_e * cos_f - cos_d * sin_f) * light_source.z;
    rotated_light_source.z =
        -light_source.x * sin_e + light_source.y * cos_e * sin_f + light_source.z * cos_e * cos_f;
    rotated_light_source.norm();

    init::init(
        &mut points,
        &mut points_color,
        &mut points_axis_range,
        spacing,
    );

    while !SIGINT_CALLED.load(Ordering::Relaxed) {
        let start = std::time::Instant::now();
        a += 0.03;
        b += 0.02;
        c += 0.01;

        let sin_a: f32 = a.sin();
        let cos_a: f32 = a.cos();

        let sin_b: f32 = b.sin();
        let cos_b: f32 = b.cos();

        let sin_c: f32 = c.sin();
        let cos_c: f32 = c.cos();
        let trig_values: Vec<f32> = vec![sin_a, cos_a, sin_b, cos_b, sin_c, cos_c];

        assert!(
            zbuffer.len() == buffer.len()
                && cbuffer.len() == buffer.len()
                && buffer_prev.len() == buffer.len()
                && cbuffer_prev.len() == buffer.len()
        );
        render_frame(
            &mut handle,
            width,
            height,
            &points,
            &points_color,
            &points_axis_range,
            &mut buffer,
            &mut buffer_prev,
            &mut cbuffer,
            &mut cbuffer_prev,
            &mut zbuffer,
            &trig_values,
            &rotated_light_source,
        );

        {
            let us_duration = start.elapsed().as_micros();
            let frame_duration_micro: u32 = 1_000_000
                / (if args.fps_limit != 0 {
                    args.fps_limit
                } else {
                    1
                });
            if args.fps_limit != 0 && us_duration < frame_duration_micro.into() {
                let diff = frame_duration_micro as u128 - us_duration;
                let sleep_dur = std::time::Duration::from_micros(diff.try_into().unwrap());
                std::thread::sleep(sleep_dur);
            }
        }

        if args.debug {
            let us_duration = start.elapsed().as_micros();
            let ms_duration = us_duration as f64 / 1000.0;
            let fps: f64 = 1_000_000.0 / (us_duration as f64);

            frame_times.push(us_duration);

            write!(
                handle,
                "{}{}{}\r",
                ansi_escape_code::SetCursorPos(1, 1 + 11),
                color::RESET,
                ansi_escape_code::EraseLineStartToCursor
            )
            .unwrap();
            write!(handle, "{fps:>8.2}fps", fps = fps).unwrap();

            write!(
                handle,
                "{}{}{}\r",
                ansi_escape_code::SetCursorPos(2, 1 + 22),
                color::RESET,
                ansi_escape_code::EraseLineStartToCursor
            )
            .unwrap();
            write!(
                handle,
                "{ms:>8.2}ms ({us:>7}us)",
                ms = ms_duration,
                us = us_duration
            )
            .unwrap();
        }

        handle.flush().expect("Error flushing handle");
    }

    handle_exit();

    if args.debug {
        println!("Width: {} | Height: {}", width, height);

        let sum: u128 = frame_times.iter().sum();
        let frames = frame_times.len();
        let frame_avg = if frames == 0 {
            sum
        } else {
            sum / frames as u128
        };
        let fps_avg = if frame_avg == 0 {
            0
        } else {
            1_000_000 / frame_avg
        };

        println!("Frame Average: {}us", frame_avg);
        println!("FPS Average: {}", fps_avg);

        println!("Points: {}", points.len());
    }
}
