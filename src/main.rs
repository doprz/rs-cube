// Copyright (c) 2023 doprz
// SPDX-License-Identifier: MIT OR Apache-2.0
use clap::Parser;
use libc::{ioctl, signal, winsize, SIGINT, STDOUT_FILENO, TIOCGWINSZ};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
mod ansi_escape_code; mod color; mod init; //additional files created for better module organization

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

#[derive(Default)]
struct Vector3f {
    x: f32,
    y: f32,
    z: f32,
}

struct AxesLuminance {
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
}

fn get_vector_mag(vec: &Vector3f) -> f32 {
    let x: f32 = vec.x;
    let y: f32 = vec.y;
    let z: f32 = vec.z;

    (x * x + y * y + z * z).sqrt()
}

fn norm_vector(vec: &mut Vector3f) {
    let x: f32 = vec.x;
    let y: f32 = vec.y;
    let z: f32 = vec.z;

    let mag: f32 = get_vector_mag(vec);
    let oomag: f32 = 1.0 / mag;

    if mag > 0.0 {
        vec.x = x * oomag;
        vec.y = y * oomag;
        vec.z = z * oomag;
    }
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

//to avoid repetiion we make this function to generate normals
fn normals(trig_values: &[f32], normal: Vector3f) -> Vector3f {
    let trig_values = &trig_values[..6];
    let (sin_a, sin_b, sin_c) = (&trig_values[0], &trig_values[2], &trig_values[4]);
    let (cos_a, cos_b, cos_c) = (&trig_values[1], &trig_values[3], &trig_values[5]);

    let x_arg1 = cos_a * sin_b * sin_c - sin_a * cos_c;
    let x_arg2 = cos_a * sin_b * cos_c + sin_a * sin_c;
    let y_arg1 = sin_a * sin_b * sin_c + cos_a * cos_c;
    let y_arg2 = sin_a * sin_b * cos_c - cos_a * sin_c;

    let normal = Vector3f{
        x: cos_a * cos_b * normal.x
            + (x_arg1) * normal.y
            + (x_arg2) * normal.z,
        y: sin_a * cos_b * normal.x
            + (y_arg1) * normal.y
            + (y_arg2) * normal.z,
        z: -normal.x * sin_b
            + normal.y * cos_b * sin_c
            + normal.z * cos_b * cos_c,
    };

    normal
}

//calculate luminance
fn lumins(normal: &Vector3f, rotated_light_source: &Vector3f) -> f32 {
        normal.x * rotated_light_source.x
        + normal.y * rotated_light_source.y
        + normal.z * rotated_light_source.z
}

//cleaner = sexier
fn get_axes_luminance(trig_values: &[f32], rotated_light_source: &Vector3f) -> AxesLuminance {
    // Axis A
    let a_surface_normal_front = Vector3f { x: 0.0, y: 0.0, z: CUBE_SIZE,};
    let a_surface_normal_back = Vector3f {x: 0.0, y: 0.0, z: -CUBE_SIZE,};

    let mut a_rotated_surface_normal_front = normals(trig_values, a_surface_normal_front);
    let mut a_rotated_surface_normal_back = normals(trig_values, a_surface_normal_back);
    norm_vector(&mut a_rotated_surface_normal_front);
    norm_vector(&mut a_rotated_surface_normal_back);
    // Axis B
    let b_surface_normal_front = Vector3f {x: 0.0, y: CUBE_SIZE, z: 0.0,};
    let b_surface_normal_back = Vector3f {x: 0.0, y: -CUBE_SIZE, z: 0.0};

    let mut b_rotated_surface_normal_front = normals(trig_values, b_surface_normal_front);
    let mut b_rotated_surface_normal_back = normals(trig_values, b_surface_normal_back);
    norm_vector(&mut b_rotated_surface_normal_front);
    norm_vector(&mut b_rotated_surface_normal_back);
    // Axis C
    let c_surface_normal_front = Vector3f {x: CUBE_SIZE, y: 0.0, z: 0.0};
    let c_surface_normal_back = Vector3f {x: -CUBE_SIZE, y: 0.0, z: 0.0,};

    let mut c_rotated_surface_normal_front = normals(trig_values, c_surface_normal_front);
    let mut c_rotated_surface_normal_back = normals(trig_values, c_surface_normal_back);
    norm_vector(&mut c_rotated_surface_normal_front);
    norm_vector(&mut c_rotated_surface_normal_back);

    AxesLuminance {
        a: (lumins(&a_rotated_surface_normal_front, rotated_light_source), 
            lumins(&a_rotated_surface_normal_back, rotated_light_source)),
        b: (lumins(&b_rotated_surface_normal_front, rotated_light_source), 
            lumins(&b_rotated_surface_normal_back, rotated_light_source)),
        c: (lumins(&c_rotated_surface_normal_front, rotated_light_source), 
            lumins(&c_rotated_surface_normal_back, rotated_light_source)),
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
    let axes_luminance = get_axes_luminance(trig_values, rotated_light_source);

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
            ).unwrap();
        } else {
            write!(
                handle,
                "{}{}{}",
                ansi_escape_code::SetCursorPos(y + 1, x + 1),
                color,
                val
            ).unwrap();
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
    unsafe { signal(SIGINT, handle_sigint as usize); }

    let args = Args::parse();
    let (mut width, mut height) = (100 as u16, 5 as u16);
    let term_size = get_term_size();

    print!("{}", ansi_escape_code::EnableAltBuffer);
    print!("{}", ansi_escape_code::EraseScreen);
    print!("{}", ansi_escape_code::CursorInvisible);

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

    //buffers
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

    let _k1: f32 = ((width as f32) * K2 * 3.0) / (8.0 * (3_f32.sqrt() * CUBE_SIZE));
    let total_frames = 10_000;
    let mut frame_times: Vec<usize> = Vec::with_capacity(total_frames);

    let mut a: f32 = -std::f32::consts::FRAC_PI_2; // Axis facing the screen (z-axis)
    let mut b: f32 = -std::f32::consts::FRAC_PI_2; // Up / Down axis (y-axis)
    let mut c: f32 = std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4; // Left / Right axis (x-axis)

    //Lighting 
    let (sin_d, sin_e, sin_f) = (0., 0., 0.); //changed since sin(0) == 0.
    let (cos_d, cos_e, cos_f) = (1., 1., 1.); //changed since cos(0) == 1.
    let light_source = Vector3f { x: 0.0, y: 0.5, z: -0.5 };
    let mut rotated_light_source = Vector3f {x: 0.0, y: 0.5, z: -0.5 };

    //added some simple maths so lighting is slighty smoother
    rotated_light_source.x = cos_d * cos_e * light_source.x
        + (cos_d * sin_e * sin_f - sin_d * cos_f) * light_source.y/a
        + (cos_d * sin_e * cos_f + sin_d * sin_f) * light_source.z/b;
    rotated_light_source.y = sin_d * cos_e * light_source.x
        + (sin_d * sin_e * sin_f + cos_d * cos_f) * light_source.y/a
        + (sin_d * sin_e * cos_f - cos_d * sin_f) * light_source.z/b ;
    rotated_light_source.z =
        -light_source.x/b * sin_e + light_source.y/a * cos_e * sin_f + light_source.z/c * cos_e * cos_f;
    norm_vector(&mut rotated_light_source);

    let spacing: f32 = 3.0 / width as f32;
    let points_size = ((CUBE_SIZE * CUBE_SIZE) / spacing).round() as usize;
    let mut points: Vec<init::Point3D> = Vec::with_capacity(points_size);
    let mut points_color: Vec<bool> = Vec::with_capacity(points_size);
    let mut points_axis_range = init::PointsAxisRange { a: 0, b: 0, c: 0 };

    init::rubik(
        &mut points,
        &mut points_color,
        &mut points_axis_range,
        spacing,
    );

    while !SIGINT_CALLED.load(Ordering::Relaxed) {
        let start = std::time::Instant::now();
        a += 0.03; b += 0.02; c += 0.01;

        let (sin_a, sin_b, sin_c) = (a.sin(), b.sin(), c.sin());
        let (cos_a, cos_b, cos_c) = (a.cos(), b.cos(), c.cos());
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
            let frame_duration_micro: u32 = 1_000_000 / 
                (if args.fps_limit != 0 {
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
            let us_duration = start.elapsed().as_micros() as usize;
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

        let (sum, frames) = (frame_times.iter().sum(), frame_times.len());
        let frame_avg: usize = if frames == 0 { sum } else {
            sum / frames
        };
        let fps_avg = if frame_avg == 0 { 0 } else {
            1_000_000 / frame_avg
        };
        println!("Frame Average: {}us\nFPS Average: {}\nPoints: {}", frame_avg, fps_avg, points.len());
    }
}
