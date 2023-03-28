use clap::Parser;
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ, signal, SIGINT};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
pub mod ansi_escape_code;

struct Point3D {
    x: f32,
    y: f32,
    z: f32,
}

struct Vector3f {
    x: f32,
    y: f32,
    z: f32,
}

struct PointsAxisRange {
    a: usize,
    b: usize,
    c: usize,
}

struct AxesLuminance {
    a: (f32, f32),
    b: (f32, f32),
    c: (f32, f32),
}

enum Axes {
    AFront,
    ABack,
    BFront,
    BBack,
    CFront,
    CBack,
}

static SIGINT_CALLED: AtomicBool = AtomicBool::new(false);

const CUBE_SIZE: f32 = 1.0; // Unit Cube
const FRAC_CUBE_SIZE_2: f32 = CUBE_SIZE / 2.0;
const FRAC_CUBE_SIZE_3: f32 = CUBE_SIZE / 3.0;

const GRID_SPACING: f32 = 0.04;
const GRID_LINE_COLOR: &str = ansi_escape_code::color::BLACK;

const K2: f32 = 10.0;

#[inline(never)]
fn get_vector_mag(vec: &Vector3f) -> f32 {
    let x: f32 = vec.x;
    let y: f32 = vec.y;
    let z: f32 = vec.z;

    (x * x + y * y + z * z).sqrt()
}

#[inline(never)]
fn norm_vector(vec: &mut Vector3f) {
    let x: f32 = vec.x;
    let y: f32 = vec.y;
    let z: f32 = vec.z;

    let mag: f32 = get_vector_mag(vec);
    // one over mag
    let oomag: f32 = 1.0 / mag;

    if mag > 0.0 {
        vec.x = x * oomag;
        vec.y = y * oomag;
        vec.z = z * oomag;
    }
}

#[inline(never)]
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

    let sin_a = trig_values[0];
    let cos_a = trig_values[1];

    let sin_b = trig_values[2];
    let cos_b = trig_values[3];

    let sin_c = trig_values[4];
    let cos_c = trig_values[5];

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

    // Luminance ranges from -1 to +1 for the dot product of the plane normal and light source normalized 3D unit vectors
    // If the luminance > 0, then the plane is facing towards the light source
    // else if luminance < 0, then the plane is facing away from the light source
    // else if luminance = 0, then the plane and the light source are perpendicular
    let luminance_index: usize = (luminance * 11.0) as usize;
    if index < index_limit && ooz > zbuffer[index] {
        zbuffer[index] = ooz;
        cbuffer[index] = if char_color {
            match axes {
                Axes::AFront => ansi_escape_code::color::YELLOW,
                Axes::ABack => ansi_escape_code::color::WHITE,
                Axes::BFront => ansi_escape_code::color::GREEN,
                Axes::BBack => ansi_escape_code::color::BLUE,
                Axes::CFront => ansi_escape_code::color::BOLD_RED,
                Axes::CBack => ansi_escape_code::color::RED,
            }
        } else {
            GRID_LINE_COLOR
        };
        buffer[index] = ".,-~:;=!*#$@".as_bytes()
            [if luminance > 0.0 { luminance_index } else { 0 }]
            as char;
    }
}

#[inline(never)]
fn init(
    points: &mut Vec<Point3D>,
    points_color: &mut Vec<bool>,
    points_axis_range: &mut PointsAxisRange,
    spacing: f32,
) {
    // Axis A
    {
        // z
        let k: f32 = FRAC_CUBE_SIZE_2;

        // y
        let mut i: f32 = -FRAC_CUBE_SIZE_2;
        while i <= FRAC_CUBE_SIZE_2 {
            // x
            let mut j: f32 = -FRAC_CUBE_SIZE_2;
            while j <= FRAC_CUBE_SIZE_2 {
                points.push(Point3D { x: i, y: j, z: k });
                points.push(Point3D { x: i, y: j, z: -k });

                let mut char_color1: bool = true;
                let mut char_color2: bool = true;
                if i > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && i < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if i > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && i < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if j > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && j < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if j > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && j < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                }

                points_color.push(char_color1);
                points_color.push(char_color2);

                j += spacing;
            }
            i += spacing;
        }

        points_axis_range.a = points.len();
    }

    // Axis B
    {
        // y
        let i: f32 = FRAC_CUBE_SIZE_2;

        // x
        let mut j: f32 = -FRAC_CUBE_SIZE_2;
        while j <= FRAC_CUBE_SIZE_2 {
            let mut k: f32 = -FRAC_CUBE_SIZE_2;
            while k <= FRAC_CUBE_SIZE_2 {
                points.push(Point3D { x: i, y: j, z: k });
                points.push(Point3D { x: -i, y: j, z: k });

                let mut char_color1: bool = true;
                let mut char_color2: bool = true;
                if j > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && j < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if j > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && j < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if k > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && k < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if k > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && k < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                }

                points_color.push(char_color1);
                points_color.push(char_color2);

                k += spacing;
            }
            j += spacing;
        }
        points_axis_range.b = points.len();
    }

    // Axis C
    {
        // x
        let j: f32 = FRAC_CUBE_SIZE_2;

        // z
        let mut k: f32 = -FRAC_CUBE_SIZE_2;
        while k <= FRAC_CUBE_SIZE_2 {
            let mut i: f32 = -FRAC_CUBE_SIZE_2;
            while i <= FRAC_CUBE_SIZE_2 {
                points.push(Point3D { x: i, y: j, z: k });
                points.push(Point3D { x: i, y: -j, z: k });

                let mut char_color1: bool = true;
                let mut char_color2: bool = true;
                if k > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && k < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if k > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && k < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if i > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && i < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                } else if i > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING
                    && i < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING
                {
                    char_color1 = false;
                    char_color2 = false;
                }

                points_color.push(char_color1);
                points_color.push(char_color2);

                i += spacing;
            }
            k += spacing;
        }
        points_axis_range.c = points.len();
    }
}

#[inline(never)]
fn get_axes_luminance(trig_values: &[f32], rotated_light_source: &Vector3f) -> AxesLuminance {
    let trig_values = &trig_values[..6];

    let sin_a = &trig_values[0];
    let cos_a = &trig_values[1];

    let sin_b = &trig_values[2];
    let cos_b = &trig_values[3];

    let sin_c = &trig_values[4];
    let cos_c = &trig_values[5];

    // Axis A
    let a_surface_normal_front = Vector3f {
        x: 0.0,
        y: 0.0,
        z: CUBE_SIZE,
    };

    let a_surface_normal_back = Vector3f {
        x: 0.0,
        y: 0.0,
        z: -CUBE_SIZE,
    };

    let x_arg1 = cos_a * sin_b * sin_c - sin_a * cos_c;
    let x_arg2 = cos_a * sin_b * cos_c + sin_a * sin_c;

    let y_arg1 = sin_a * sin_b * sin_c + cos_a * cos_c;
    let y_arg2 = sin_a * sin_b * cos_c - cos_a * sin_c;

    let mut a_rotated_surface_normal_front = Vector3f {
        x: cos_a * cos_b * a_surface_normal_front.x
            + (x_arg1) * a_surface_normal_front.y
            + (x_arg2) * a_surface_normal_front.z,
        y: sin_a * cos_b * a_surface_normal_front.x
            + (y_arg1) * a_surface_normal_front.y
            + (y_arg2) * a_surface_normal_front.z,
        z: -a_surface_normal_front.x * sin_b
            + a_surface_normal_front.y * cos_b * sin_c
            + a_surface_normal_front.z * cos_b * cos_c,
    };
    norm_vector(&mut a_rotated_surface_normal_front);

    let mut a_rotated_surface_normal_back = Vector3f {
        x: cos_a * cos_b * a_surface_normal_back.x
            + (x_arg1) * a_surface_normal_back.y
            + (x_arg2) * a_surface_normal_back.z,
        y: sin_a * cos_b * a_surface_normal_back.x
            + (y_arg1) * a_surface_normal_back.y
            + (y_arg2) * a_surface_normal_back.z,
        z: -a_surface_normal_back.x * sin_b
            + a_surface_normal_back.y * cos_b * sin_c
            + a_surface_normal_back.z * cos_b * cos_c,
    };
    norm_vector(&mut a_rotated_surface_normal_back);

    let a_luminance_front: f32 = a_rotated_surface_normal_front.x * rotated_light_source.x
        + a_rotated_surface_normal_front.y * rotated_light_source.y
        + a_rotated_surface_normal_front.z * rotated_light_source.z;
    let a_luminance_back: f32 = a_rotated_surface_normal_back.x * rotated_light_source.x
        + a_rotated_surface_normal_back.y * rotated_light_source.y
        + a_rotated_surface_normal_back.z * rotated_light_source.z;

    // Axis B
    let b_surface_normal_front = Vector3f {
        x: 0.0,
        y: CUBE_SIZE,
        z: 0.0,
    };

    let b_surface_normal_back = Vector3f {
        x: 0.0,
        y: -CUBE_SIZE,
        z: 0.0,
    };

    let mut b_rotated_surface_normal_front = Vector3f {
        x: cos_a * cos_b * b_surface_normal_front.x
            + (x_arg1) * b_surface_normal_front.y
            + (x_arg2) * b_surface_normal_front.z,
        y: sin_a * cos_b * b_surface_normal_front.x
            + (y_arg1) * b_surface_normal_front.y
            + (y_arg2) * b_surface_normal_front.z,
        z: -b_surface_normal_front.x * sin_b
            + b_surface_normal_front.y * cos_b * sin_c
            + b_surface_normal_front.z * cos_b * cos_c,
    };
    norm_vector(&mut b_rotated_surface_normal_front);

    let mut b_rotated_surface_normal_back = Vector3f {
        x: cos_a * cos_b * b_surface_normal_back.x
            + (x_arg1) * b_surface_normal_back.y
            + (x_arg2) * b_surface_normal_back.z,
        y: sin_a * cos_b * b_surface_normal_back.x
            + (y_arg1) * b_surface_normal_back.y
            + (y_arg2) * b_surface_normal_back.z,
        z: -b_surface_normal_back.x * sin_b
            + b_surface_normal_back.y * cos_b * sin_c
            + b_surface_normal_back.z * cos_b * cos_c,
    };
    norm_vector(&mut b_rotated_surface_normal_back);

    let b_luminance_front: f32 = b_rotated_surface_normal_front.x * rotated_light_source.x
        + b_rotated_surface_normal_front.y * rotated_light_source.y
        + b_rotated_surface_normal_front.z * rotated_light_source.z;
    let b_luminance_back: f32 = b_rotated_surface_normal_back.x * rotated_light_source.x
        + b_rotated_surface_normal_back.y * rotated_light_source.y
        + b_rotated_surface_normal_back.z * rotated_light_source.z;

    // Axis C
    let c_surface_normal_front = Vector3f {
        x: CUBE_SIZE,
        y: 0.0,
        z: 0.0,
    };

    let c_surface_normal_back = Vector3f {
        x: -CUBE_SIZE,
        y: 0.0,
        z: 0.0,
    };

    let mut c_rotated_surface_normal_front = Vector3f {
        x: cos_a * cos_b * c_surface_normal_front.x
            + (x_arg1) * c_surface_normal_front.y
            + (x_arg2) * c_surface_normal_front.z,
        y: sin_a * cos_b * c_surface_normal_front.x
            + (y_arg1) * c_surface_normal_front.y
            + (y_arg2) * c_surface_normal_front.z,
        z: -c_surface_normal_front.x * sin_b
            + c_surface_normal_front.y * cos_b * sin_c
            + c_surface_normal_front.z * cos_b * cos_c,
    };
    norm_vector(&mut c_rotated_surface_normal_front);

    let mut c_rotated_surface_normal_back = Vector3f {
        x: cos_a * cos_b * c_surface_normal_back.x
            + (x_arg1) * c_surface_normal_back.y
            + (x_arg2) * c_surface_normal_back.z,
        y: sin_a * cos_b * c_surface_normal_back.x
            + (y_arg1) * c_surface_normal_back.y
            + (y_arg2) * c_surface_normal_back.z,
        z: -c_surface_normal_back.x * sin_b
            + c_surface_normal_back.y * cos_b * sin_c
            + c_surface_normal_back.z * cos_b * cos_c,
    };
    norm_vector(&mut c_rotated_surface_normal_back);

    let c_luminance_front: f32 = c_rotated_surface_normal_front.x * rotated_light_source.x
        + c_rotated_surface_normal_front.y * rotated_light_source.y
        + c_rotated_surface_normal_front.z * rotated_light_source.z;
    let c_luminance_back: f32 = c_rotated_surface_normal_back.x * rotated_light_source.x
        + c_rotated_surface_normal_back.y * rotated_light_source.y
        + c_rotated_surface_normal_back.z * rotated_light_source.z;

    AxesLuminance {
        a: (a_luminance_front, a_luminance_back),
        b: (b_luminance_front, b_luminance_back),
        c: (c_luminance_front, c_luminance_back),
    }
}

#[inline(never)]
fn render_frame<'a>(
    mut handle: impl Write,
    width: u16,
    height: u16,
    points: &[Point3D],
    points_color: &[bool],
    points_axis_range: &PointsAxisRange,
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
    cbuffer.fill(ansi_escape_code::color::RESET);
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
    let mut prev_set_color: &str = ansi_escape_code::color::RESET;

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

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Enable debug info
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    #[arg(short, long, default_value_t = 60u32)]
    fps_limit: u32,
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
    print!("{}", ansi_escape_code::color::RESET);
    print!("{}", ansi_escape_code::CursorVisible);
}

fn handle_sigint() {
    SIGINT_CALLED.store(true, Ordering::Relaxed);
}

#[inline(never)]
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

    let mut cbuffer: Vec<&str> = vec![ansi_escape_code::color::RESET; (width * height).into()];
    let mut cbuffer_prev: Vec<&str> = vec![ansi_escape_code::color::RESET; (width * height).into()];

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
    let mut points: Vec<Point3D> = Vec::with_capacity(points_size);
    let mut points_color: Vec<bool> = Vec::with_capacity(points_size);
    let mut points_axis_range = PointsAxisRange { a: 0, b: 0, c: 0 };

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
    norm_vector(&mut rotated_light_source);

    init(
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
            let frame_duration_micro: u32 = 1_000_000 / (if args.fps_limit != 0 { args.fps_limit } else { 1 });
            if args.fps_limit != 0  && us_duration < frame_duration_micro.into() {
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
                ansi_escape_code::SetCursorPos(1, 1 + 24),
                ansi_escape_code::color::RESET,
                ansi_escape_code::EraseLineStartToCursor
                )
                .unwrap();
            write!(handle, "{fps:>7.2}fps", fps = fps).unwrap();

            write!(
                handle,
                "{}{}{}\r",
                ansi_escape_code::SetCursorPos(2, 1 + 24),
                ansi_escape_code::color::RESET,
                ansi_escape_code::EraseLineStartToCursor
                )
                .unwrap();
            write!(
                handle,
                "{ms:>7.2}ms ({us:>7}us)",
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
