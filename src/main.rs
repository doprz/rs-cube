mod ANSI_escape_code;

use std::io::{self, Write};
use std::time::{Duration, Instant};
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};

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

const FPS_LIMIT: f32 = 60.0;
const FRAME_DURATION_MICRO: f32 = 1_000_000.0 / (if FPS_LIMIT != 0.0 {FPS_LIMIT} else {1.0});

const CUBE_SIZE: f32 = 1.0; // Unit Cube
const FRAC_CUBE_SIZE_2: f32 = CUBE_SIZE / 2.0;
const FRAC_CUBE_SIZE_3: f32 = CUBE_SIZE / 3.0;
const FRAC_CUBE_SIZE_4: f32 = CUBE_SIZE / 4.0;

const GRID_SPACING: f32 = 0.04;
const GRID_LINE_COLOR: &str = ANSI_escape_code::color::BLACK;

const K2: f32 = 10.0;


fn get_vector_mag(vec: &Vector3f) -> f32 {
    let x: f32 = vec.x;
    let y: f32 = vec.y;
    let z: f32 = vec.z;

    (x*x + y*y + z*z).sqrt()
}

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

fn update_buffers<'a>(i: f32, j: f32, k: f32, width: u16, height: u16, buffer: &mut [char], zbuffer: &mut [f32], cbuffer: &mut [&'a str], trig_values: &[f32] , char_color: &'a str, luminance: f32) {
    assert!(zbuffer.len() == buffer.len() && cbuffer.len() == buffer.len());
    let trig_values = &trig_values[..6];

    let sin_a = trig_values[0];
    let cos_a = trig_values[1];

    let sin_b = trig_values[2];
    let cos_b = trig_values[3];

    let sin_c = trig_values[4];
    let cos_c = trig_values[5];

    let x: f32 = cos_a*cos_b*j + (cos_a*sin_b*sin_c - sin_a*cos_c)*i + (cos_a*sin_b*cos_c + sin_a*sin_c)*k;
    let y: f32 = sin_a*cos_b*j + (sin_a*sin_b*sin_c + cos_a*cos_c)*i + (sin_a*sin_b*cos_c - cos_a*sin_c)*k;
    let z: f32 = -j*sin_b + i*cos_b*sin_c + k*cos_b*cos_c + K2;

    let ooz: f32 = 1.0 / z; // "One over z"

    let k1: f32 = ((width as f32) * K2 * 3.0) / (8.0 * ((3.0_f32).sqrt() * CUBE_SIZE));
    let xp: u32 = (((width as f32)/(2.0)) + (k1*ooz*x)) as u32;
    let yp: u32 = (((height as f32)/(2.0)) - (k1*ooz*y)) as u32;

    let index: usize = (xp + yp * width as u32).try_into().unwrap();
    let index_limit: usize = buffer.len();

    // Luminance ranges from -1 to +1 for the dot product of the plane normal and light source normalized 3D unit vectors
    // If the luminance > 0, then the plane is facing towards the light source
    // else if luminance < 0, then the plane is facing away from the light source
    // else if luminance = 0, then the plane and the light source are perpendicular
    let luminance_index: usize = (luminance * 11.0) as usize;
    if index < index_limit {
        if ooz > zbuffer[index] {
            zbuffer[index] = ooz;
            cbuffer[index] = char_color;
            buffer[index] = ".,-~:;=!*#$@".as_bytes()[if luminance > 0.0 {luminance_index} else {0}] as char;
        }
    }
}

// fn init(points: &mut [Point3D], width: usize) {
// fn init(points: &mut Vec<Point3D>, size: usize, width: usize) {
fn init(points: &mut Vec<Point3D>, points_color: &mut Vec<&str>, points_axis_range: &mut PointsAxisRange, spacing: f32) {
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
                points.push(Point3D {
                    x: i,
                    y: j,
                    z: k,
                });
                points.push(Point3D {
                    x: i,
                    y: j,
                    z: -k,
                });

                let mut char_color1: &str = ANSI_escape_code::color::YELLOW;
                let mut char_color2: &str = ANSI_escape_code::color::WHITE;
                if i > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        i < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if i > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        i < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if j > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        j < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if j > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        j < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
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
                points.push(Point3D {
                    x: i,
                    y: j,
                    z: k,
                });
                points.push(Point3D {
                    x: -i,
                    y: j,
                    z: k,
                });

                let mut char_color1: &str = ANSI_escape_code::color::GREEN;
                let mut char_color2: &str = ANSI_escape_code::color::BLUE;
                if j > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        j < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if j > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        j < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if k > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        k < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if k > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        k < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
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
                points.push(Point3D {
                    x: i,
                    y: j,
                    z: k,
                });
                points.push(Point3D {
                    x: i,
                    y: -j,
                    z: k,
                });

                let mut char_color1: &str = ANSI_escape_code::color::BOLD_RED;
                let mut char_color2: &str = ANSI_escape_code::color::RED;
                if k > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        k < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if k > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        k < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if i > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        i < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
                } else if i > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                        i < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING {
                    char_color1 = GRID_LINE_COLOR;
                    char_color2 = GRID_LINE_COLOR;
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

    let mut a_rotated_surface_normal_front = Vector3f {
        x: cos_a*cos_b*a_surface_normal_front.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*a_surface_normal_front.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*a_surface_normal_front.z,
        y: sin_a*cos_b*a_surface_normal_front.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*a_surface_normal_front.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*a_surface_normal_front.z,
        z: -a_surface_normal_front.x*sin_b + a_surface_normal_front.y*cos_b*sin_c + a_surface_normal_front.z*cos_b*cos_c
    };
    norm_vector(&mut a_rotated_surface_normal_front);

    let mut a_rotated_surface_normal_back = Vector3f {
        x: cos_a*cos_b*a_surface_normal_back.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*a_surface_normal_back.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*a_surface_normal_back.z,
        y: sin_a*cos_b*a_surface_normal_back.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*a_surface_normal_back.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*a_surface_normal_back.z,
        z: -a_surface_normal_back.x*sin_b + a_surface_normal_back.y*cos_b*sin_c + a_surface_normal_back.z*cos_b*cos_c
    };
    norm_vector(&mut a_rotated_surface_normal_back);

    let a_luminance_front: f32 = a_rotated_surface_normal_front.x*rotated_light_source.x + a_rotated_surface_normal_front.y*rotated_light_source.y + a_rotated_surface_normal_front.z*rotated_light_source.z;
    let a_luminance_back: f32 = a_rotated_surface_normal_back.x*rotated_light_source.x + a_rotated_surface_normal_back.y*rotated_light_source.y + a_rotated_surface_normal_back.z*rotated_light_source.z;

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
        x: cos_a*cos_b*b_surface_normal_front.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*b_surface_normal_front.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*b_surface_normal_front.z,
        y: sin_a*cos_b*b_surface_normal_front.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*b_surface_normal_front.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*b_surface_normal_front.z,
        z: -b_surface_normal_front.x*sin_b + b_surface_normal_front.y*cos_b*sin_c + b_surface_normal_front.z*cos_b*cos_c
    };
    norm_vector(&mut b_rotated_surface_normal_front);

    let mut b_rotated_surface_normal_back = Vector3f {
        x: cos_a*cos_b*b_surface_normal_back.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*b_surface_normal_back.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*b_surface_normal_back.z,
        y: sin_a*cos_b*b_surface_normal_back.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*b_surface_normal_back.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*b_surface_normal_back.z,
        z: -b_surface_normal_back.x*sin_b + b_surface_normal_back.y*cos_b*sin_c + b_surface_normal_back.z*cos_b*cos_c
    };
    norm_vector(&mut b_rotated_surface_normal_back);

    let b_luminance_front: f32 = b_rotated_surface_normal_front.x*rotated_light_source.x + b_rotated_surface_normal_front.y*rotated_light_source.y + b_rotated_surface_normal_front.z*rotated_light_source.z;
    let b_luminance_back: f32 = b_rotated_surface_normal_back.x*rotated_light_source.x + b_rotated_surface_normal_back.y*rotated_light_source.y + b_rotated_surface_normal_back.z*rotated_light_source.z;

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
        x: cos_a*cos_b*c_surface_normal_front.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*c_surface_normal_front.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*c_surface_normal_front.z,
        y: sin_a*cos_b*c_surface_normal_front.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*c_surface_normal_front.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*c_surface_normal_front.z,
        z: -c_surface_normal_front.x*sin_b + c_surface_normal_front.y*cos_b*sin_c + c_surface_normal_front.z*cos_b*cos_c
    };
    norm_vector(&mut c_rotated_surface_normal_front);

    let mut c_rotated_surface_normal_back = Vector3f {
        x: cos_a*cos_b*c_surface_normal_back.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*c_surface_normal_back.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*c_surface_normal_back.z,
        y: sin_a*cos_b*c_surface_normal_back.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*c_surface_normal_back.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*c_surface_normal_back.z,
        z: -c_surface_normal_back.x*sin_b + c_surface_normal_back.y*cos_b*sin_c + c_surface_normal_back.z*cos_b*cos_c
    };
    norm_vector(&mut c_rotated_surface_normal_back);

    let c_luminance_front: f32 = c_rotated_surface_normal_front.x*rotated_light_source.x + c_rotated_surface_normal_front.y*rotated_light_source.y + c_rotated_surface_normal_front.z*rotated_light_source.z;
    let c_luminance_back: f32 = c_rotated_surface_normal_back.x*rotated_light_source.x + c_rotated_surface_normal_back.y*rotated_light_source.y + c_rotated_surface_normal_back.z*rotated_light_source.z;

    AxesLuminance {
        a: (a_luminance_front, a_luminance_back),
        b: (b_luminance_front, b_luminance_back),
        c: (c_luminance_front, c_luminance_back),
    }
}

#[inline(never)]
fn render_frame<'a>(mut handle: impl Write, width: u16, height: u16, points: &[Point3D], points_color: &[&'a str], points_axis_range: &PointsAxisRange, buffer: &mut [char], buffer_prev: &mut [char], cbuffer: &mut [&'a str], cbuffer_prev: &mut [&'a str], zbuffer: &mut [f32], trig_values: &[f32], rotated_light_source: &Vector3f) {
    buffer_prev.copy_from_slice(buffer);
    cbuffer_prev.copy_from_slice(cbuffer);

    buffer.fill(' ');
    cbuffer.fill(ANSI_escape_code::color::RESET);
    zbuffer.fill(0.0);

    let points_color = &points_color[..points.len()];

    let axes_luminance = get_axes_luminance(trig_values, rotated_light_source);
    write!(handle, "{}{}{}\r", ANSI_escape_code::set_cursor_pos(3, 1 + 24), ANSI_escape_code::color::RESET, ANSI_escape_code::EraseLineStartToCursor).unwrap();
    write!(handle, "Points: {}", points.len()).unwrap();

    // for (index, point) in points.iter().enumerate() {
    for index in 0..points.len() {
        let point = &points[index];
        let color = points_color[index];
        let luminance = if index < points_axis_range.a {
            if index & 1 == 0 {
                axes_luminance.a.0
            } else {
                axes_luminance.a.1
            }
        } else if index < points_axis_range.b {
            if index & 1 == 0 {
                axes_luminance.b.0
            } else {
                axes_luminance.b.1
            }
        } else {
            if index & 1 == 0 {
                axes_luminance.c.0
            } else {
                axes_luminance.c.1
            }
        };
        update_buffers(point.x, point.y, point.z, width, height, buffer, zbuffer, cbuffer, trig_values, color, luminance);
    }


    // write!(handle, "{}", ANSI_escape_code::SetCursorHome).unwrap();

    let mut prev_set_color: &str = ANSI_escape_code::color::RESET;

    // for (index, val) in buffer.iter().enumerate() {
    for index in 0..buffer.len() {
        let val = buffer[index];
        let color = cbuffer[index];
        if (val == buffer_prev[index]) && (color == cbuffer_prev[index]) {
            continue;
        }

        let x: u32 = (index % width as usize).try_into().unwrap();
        let y: u32 = (index / width as usize).try_into().unwrap();

        // Move cursor, add color, and print char
        if color == prev_set_color {
            write!(handle, "{}{}", ANSI_escape_code::set_cursor_pos(y+1, x+1), val).unwrap();
        } else {
            write!(handle, "{}{}{}", ANSI_escape_code::set_cursor_pos(y+1, x+1), color, val).unwrap();
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
    print!("{}", ANSI_escape_code::EraseScreen);
    print!("{}", ANSI_escape_code::DisableAltBuffer);
    print!("{}", ANSI_escape_code::color::RESET);
    print!("{}", ANSI_escape_code::CursorVisible);
}

fn main() {
    print!("{}", ANSI_escape_code::EnableAltBuffer);
    print!("{}", ANSI_escape_code::EraseScreen);
    print!("{}", ANSI_escape_code::CursorInvisible);

    let mut width: u16 = 100;
    let mut height: u16 = 50;

    let term_size = get_term_size();

    match term_size {
        Ok(s) => {
            width = s.0;
            height = s.1;
        },
        Err(err) => {
            println!("{}", err)
        }
    }

    let stdout = io::stdout(); // get the global stdout entity
    // optional: wrap that handle in a buffer and aquire a lock on it
    // let mut handle = io::BufWriter::new(stdout.lock());
    let mut handle = io::BufWriter::with_capacity((width*height*3).into(), stdout.lock());

    let mut buffer: Vec<char> = vec![' '; (width * height).into()];
    let mut buffer_prev: Vec<char> = vec![' '; (width * height).into()];

    let mut cbuffer: Vec<&str> = vec![ANSI_escape_code::color::RESET; (width * height).into()];
    let mut cbuffer_prev: Vec<&str> = vec![ANSI_escape_code::color::RESET; (width * height).into()];

    let mut zbuffer: Vec<f32> = vec![0.0; (width * height).into()];

    assert!(zbuffer.len() == buffer.len() && cbuffer.len() == buffer.len() && buffer_prev.len() == buffer.len() && cbuffer_prev.len() == buffer.len());

    let mut spacing: f32 = 3.0 / width as f32;
    let mut k1: f32 = ((width as f32) * (K2 as f32) * 3.0) / (8.0 * ((3 as f32).sqrt() * CUBE_SIZE as f32));

    let points_size = ((CUBE_SIZE * CUBE_SIZE) / spacing).round() as usize;
    let mut points: Vec<Point3D> = Vec::with_capacity(points_size);
    let mut points_color: Vec<&str> = Vec::with_capacity(points_size);
    let mut points_axis_range = PointsAxisRange {
        a: 0,
        b: 0,
        c: 0,
    };

    let light_source = Vector3f {
        x: 0.0, 
        y: 1.0, 
        z: -1.0
    };
    let mut rotated_light_source = Vector3f {
        x: 0.0, 
        y: 1.0, 
        z: -1.0
    };

    let total_frames = 1_000;
    let mut frame_times: Vec<u128> = Vec::with_capacity(total_frames);

    let mut a: f32 = -std::f32::consts::FRAC_PI_2; // Axis facing the screen (z-axis)
    let mut b: f32 = -std::f32::consts::FRAC_PI_2; // Up / Down axis (y-axis)
    let mut c: f32 = std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4; // Left / Right axis (x-axis)

    let mut sin_a: f32 = a.sin();
    let mut cos_a: f32 = a.cos();

    let mut sin_b: f32 = b.sin();
    let mut cos_b: f32 = b.cos();

    let mut sin_c: f32 = c.sin();
    let mut cos_c: f32 = c.cos();
    let mut trig_values: Vec<f32> = vec![sin_a, cos_a, sin_b, cos_b, sin_c, cos_c];

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


    rotated_light_source.x = cos_d*cos_e*light_source.x + (cos_d*sin_e*sin_f - sin_d*cos_f)*light_source.y + (cos_d*sin_e*cos_f + sin_d*sin_f)*light_source.z;
    rotated_light_source.y = sin_d*cos_e*light_source.x + (sin_d*sin_e*sin_f + cos_d*cos_f)*light_source.y + (sin_d*sin_e*cos_f - cos_d*sin_f)*light_source.z;
    rotated_light_source.z = -light_source.x*sin_e + light_source.y*cos_e*sin_f + light_source.z*cos_e*cos_f;
    norm_vector(&mut rotated_light_source);

    init(&mut points, &mut points_color, &mut points_axis_range, spacing);
    
    // loop {
    for _ in 0..total_frames {
        let start = Instant::now();
        a += 0.03;
        b += 0.02;
        c += 0.01;

        sin_a = a.sin();
        cos_a = a.cos();

        sin_b = b.sin();
        cos_b = b.cos();

        sin_c = c.sin();
        cos_c = c.cos();
        trig_values = [sin_a, cos_a, sin_b, cos_b, sin_c, cos_c].to_vec();

        assert!(zbuffer.len() == buffer.len() && cbuffer.len() == buffer.len() && buffer_prev.len() == buffer.len() && cbuffer_prev.len() == buffer.len());
        render_frame(&mut handle, width, height, &points, &points_color, &points_axis_range, &mut buffer, &mut buffer_prev, &mut cbuffer, &mut cbuffer_prev, &mut zbuffer, &trig_values, &rotated_light_source);
        // handle.flush().expect("Error flushing handle");

        // let sleep_dur = std::time::Duration::from_millis(200);
        // std::thread::sleep(sleep_dur);

        let us_duration = start.elapsed().as_micros();
        let ms_duration = us_duration as f64 / 1000.0;
        let fps: f64 = 1_000_000.0 / (us_duration as f64);

        frame_times.push(us_duration);

        write!(handle, "{}{}{}\r", ANSI_escape_code::set_cursor_pos(1, 1 + 24), ANSI_escape_code::color::RESET, ANSI_escape_code::EraseLineStartToCursor).unwrap();
        write!(handle, "{fps:>7.2}fps", fps=fps).unwrap();

        write!(handle, "{}{}{}\r", ANSI_escape_code::set_cursor_pos(2, 1 + 24), ANSI_escape_code::color::RESET, ANSI_escape_code::EraseLineStartToCursor).unwrap();
        write!(handle, "{ms:>7.2}ms ({us:>7}us)", ms=ms_duration, us=us_duration).unwrap();
        handle.flush().expect("Error flushing handle");

        // let sleep_dur = std::time::Duration::from_millis(25);
        // std::thread::sleep(sleep_dur);
    }
    
    handle_exit();

    println!("Width: {} | Height: {}", width, height);

    let sum: u128 = frame_times.iter().sum();
    let frames = frame_times.len();
    let frame_avg = sum / frames as u128;

    println!("Frame Average: {}us", frame_avg);
    println!("FPS Average: {}", 1_000_000 / frame_avg);

    println!("Points: {}", points.len());
}
