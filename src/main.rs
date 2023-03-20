mod ANSI_escape_code;

use std::io::{self, Write};
use std::time::{Duration, Instant};
use libc::{ioctl, winsize, STDOUT_FILENO, TIOCGWINSZ};

struct Vector3f {
    x: f32,
    y: f32,
    z: f32,
}

const FPS_LIMIT: f32 = 60.0;
const FRAME_DURATION_MICRO: f32 = 1_000_000.0 / (if FPS_LIMIT != 0.0 {FPS_LIMIT} else {1.0});

const CUBE_SIZE: f32 = 1.0; // Unit Cube
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

    if (mag > 0.0) {
        vec.x = x * oomag;
        vec.y = y * oomag;
        vec.z = z * oomag;
    }
}

fn update_buffers<'a>(i: f32, j: f32, k: f32, width: u16, height: u16, buffer: &mut [char], zbuffer: &mut [f32], cbuffer: &mut [&'a str], trig_values: &[f32] , char_color: &'a str, luminance: f32) {
    let sin_a = &trig_values[0];
    let cos_a = &trig_values[1];

    let sin_b = &trig_values[2];
    let cos_b = &trig_values[3];

    let sin_c = &trig_values[4];
    let cos_c = &trig_values[5];

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
    if (index >= 0 && index < index_limit) {
        if (ooz > zbuffer[index]) {
            zbuffer[index] = ooz;
            cbuffer[index] = char_color;
            buffer[index] = ".,-~:;=!*#$@".as_bytes()[if luminance > 0.0 {luminance_index} else {0}] as char;
        }
    }
}

fn render_cube_axis_a<'a>(width: u16, height: u16, buffer: &mut [char], cbuffer: &mut [&'a str], zbuffer: &mut [f32], trig_values: &[f32], spacing: f32, rotated_light_source: &Vector3f, color1: &'a str, color2: &'a str) {
    let sin_a = &trig_values[0];
    let cos_a = &trig_values[1];

    let sin_b = &trig_values[2];
    let cos_b = &trig_values[3];

    let sin_c = &trig_values[4];
    let cos_c = &trig_values[5];

    let surface_normal_front = Vector3f {
        x: 0.0,
        y: 0.0,
        z: CUBE_SIZE,
    };

    let surface_normal_back = Vector3f {
        x: 0.0,
        y: 0.0,
        z: -CUBE_SIZE,
    };

    let mut rotated_surface_normal_front = Vector3f {
        x: cos_a*cos_b*surface_normal_front.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*surface_normal_front.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*surface_normal_front.z,
        y: sin_a*cos_b*surface_normal_front.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*surface_normal_front.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*surface_normal_front.z,
        z: -surface_normal_front.x*sin_b + surface_normal_front.y*cos_b*sin_c + surface_normal_front.z*cos_b*cos_c
    };
    norm_vector(&mut rotated_surface_normal_front);

    let mut rotated_surface_normal_back = Vector3f {
        x: cos_a*cos_b*surface_normal_back.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*surface_normal_back.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*surface_normal_back.z,
        y: sin_a*cos_b*surface_normal_back.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*surface_normal_back.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*surface_normal_back.z,
        z: -surface_normal_back.x*sin_b + surface_normal_back.y*cos_b*sin_c + surface_normal_back.z*cos_b*cos_c
    };
    norm_vector(&mut rotated_surface_normal_back);

    let luminance_front: f32 = rotated_surface_normal_front.x*rotated_light_source.x + rotated_surface_normal_front.y*rotated_light_source.y + rotated_surface_normal_front.z*rotated_light_source.z;
    let luminance_back: f32 = rotated_surface_normal_back.x*rotated_light_source.x + rotated_surface_normal_back.y*rotated_light_source.y + rotated_surface_normal_back.z*rotated_light_source.z;

    // z
    let k: f32 = CUBE_SIZE / 2.0;

    // y
    let mut i: f32 = -CUBE_SIZE / 2.0;
    while i <= CUBE_SIZE / 2.0 {
        // x
        let mut j: f32 = -CUBE_SIZE / 2.0;
        while j <= CUBE_SIZE / 2.0 {
            let mut char_color1: &str = &color1;
            let mut char_color2: &str = &color2;
            if (i > (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) - GRID_SPACING &&
                    i < (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (i > (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) - GRID_SPACING &&
                    i < (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (j > (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) - GRID_SPACING &&
                    j < (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (j > (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) - GRID_SPACING &&
                    j < (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            }

            // Front Face
            update_buffers(i, j, k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color1, luminance_front);

            // Back Face
            update_buffers(i, j, -k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color2, luminance_back);

            j += spacing;
        }
        i += spacing;
    }
}

fn render_cube_axis_b<'a>(width: u16, height: u16, buffer: &mut [char], cbuffer: &mut [&'a str], zbuffer: &mut [f32], trig_values: &[f32], spacing: f32, rotated_light_source: &Vector3f, color1: &'a str, color2: &'a str) {
    let sin_a = &trig_values[0];
    let cos_a = &trig_values[1];

    let sin_b = &trig_values[2];
    let cos_b = &trig_values[3];

    let sin_c = &trig_values[4];
    let cos_c = &trig_values[5];

    let surface_normal_front = Vector3f {
        x: 0.0,
        y: 0.0,
        z: CUBE_SIZE,
    };

    let surface_normal_back = Vector3f {
        x: 0.0,
        y: 0.0,
        z: -CUBE_SIZE,
    };

    let mut rotated_surface_normal_front = Vector3f {
        x: cos_a*cos_b*surface_normal_front.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*surface_normal_front.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*surface_normal_front.z,
        y: sin_a*cos_b*surface_normal_front.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*surface_normal_front.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*surface_normal_front.z,
        z: -surface_normal_front.x*sin_b + surface_normal_front.y*cos_b*sin_c + surface_normal_front.z*cos_b*cos_c
    };
    norm_vector(&mut rotated_surface_normal_front);

    let mut rotated_surface_normal_back = Vector3f {
        x: cos_a*cos_b*surface_normal_back.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*surface_normal_back.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*surface_normal_back.z,
        y: sin_a*cos_b*surface_normal_back.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*surface_normal_back.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*surface_normal_back.z,
        z: -surface_normal_back.x*sin_b + surface_normal_back.y*cos_b*sin_c + surface_normal_back.z*cos_b*cos_c
    };
    norm_vector(&mut rotated_surface_normal_back);

    let luminance_front: f32 = rotated_surface_normal_front.x*rotated_light_source.x + rotated_surface_normal_front.y*rotated_light_source.y + rotated_surface_normal_front.z*rotated_light_source.z;
    let luminance_back: f32 = rotated_surface_normal_back.x*rotated_light_source.x + rotated_surface_normal_back.y*rotated_light_source.y + rotated_surface_normal_back.z*rotated_light_source.z;

    // y
    let i: f32 = -CUBE_SIZE / 2.0;

    // x
    let mut j: f32 = -CUBE_SIZE / 2.0;
    while j <= CUBE_SIZE / 2.0 {
        let mut k: f32 = -CUBE_SIZE / 2.0;
        while k <= CUBE_SIZE / 2.0 {
            let mut char_color1: &str = &color1;
            let mut char_color2: &str = &color2;
            if (j > (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) - GRID_SPACING &&
                    j < (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (j > (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) - GRID_SPACING &&
                    j < (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (k > (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) - GRID_SPACING &&
                    k < (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (k > (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) - GRID_SPACING &&
                    k < (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            }

            // Front Face
            update_buffers(i, j, k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color1, luminance_front);

            // Back Face
            update_buffers(-i, j, k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color2, luminance_back);

            k += spacing;
        }
        j += spacing;
    }
}

fn render_cube_axis_c<'a>(width: u16, height: u16, buffer: &mut [char], cbuffer: &mut [&'a str], zbuffer: &mut [f32], trig_values: &[f32], spacing: f32, rotated_light_source: &Vector3f, color1: &'a str, color2: &'a str) {
    let sin_a = &trig_values[0];
    let cos_a = &trig_values[1];

    let sin_b = &trig_values[2];
    let cos_b = &trig_values[3];

    let sin_c = &trig_values[4];
    let cos_c = &trig_values[5];

    let surface_normal_front = Vector3f {
        x: 0.0,
        y: 0.0,
        z: CUBE_SIZE,
    };

    let surface_normal_back = Vector3f {
        x: 0.0,
        y: 0.0,
        z: -CUBE_SIZE,
    };

    let mut rotated_surface_normal_front = Vector3f {
        x: cos_a*cos_b*surface_normal_front.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*surface_normal_front.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*surface_normal_front.z,
        y: sin_a*cos_b*surface_normal_front.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*surface_normal_front.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*surface_normal_front.z,
        z: -surface_normal_front.x*sin_b + surface_normal_front.y*cos_b*sin_c + surface_normal_front.z*cos_b*cos_c
    };
    norm_vector(&mut rotated_surface_normal_front);

    let mut rotated_surface_normal_back = Vector3f {
        x: cos_a*cos_b*surface_normal_back.x + (cos_a*sin_b*sin_c - sin_a*cos_c)*surface_normal_back.y + (cos_a*sin_b*cos_c + sin_a*sin_c)*surface_normal_back.z,
        y: sin_a*cos_b*surface_normal_back.x + (sin_a*sin_b*sin_c + cos_a*cos_c)*surface_normal_back.y + (sin_a*sin_b*cos_c - cos_a*sin_c)*surface_normal_back.z,
        z: -surface_normal_back.x*sin_b + surface_normal_back.y*cos_b*sin_c + surface_normal_back.z*cos_b*cos_c
    };
    norm_vector(&mut rotated_surface_normal_back);

    let luminance_front: f32 = rotated_surface_normal_front.x*rotated_light_source.x + rotated_surface_normal_front.y*rotated_light_source.y + rotated_surface_normal_front.z*rotated_light_source.z;
    let luminance_back: f32 = rotated_surface_normal_back.x*rotated_light_source.x + rotated_surface_normal_back.y*rotated_light_source.y + rotated_surface_normal_back.z*rotated_light_source.z;

    // x
    let j: f32 = -CUBE_SIZE / 2.0;

    // z
    let mut k: f32 = -CUBE_SIZE / 2.0;
    while k <= CUBE_SIZE / 2.0 {
        let mut i: f32 = -CUBE_SIZE / 2.0;
        while i <= CUBE_SIZE / 2.0 {
            let mut char_color1: &str = &color1;
            let mut char_color2: &str = &color2;
            
            if (k > (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) - GRID_SPACING &&
                    k < (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (k > (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) - GRID_SPACING &&
                    k < (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (i > (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) - GRID_SPACING &&
                    i < (-CUBE_SIZE/2.0 + CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (i > (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) - GRID_SPACING &&
                    i < (CUBE_SIZE/2.0 - CUBE_SIZE/3.0) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            }

            // Front Face
            update_buffers(i, j, k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color1, luminance_front);

            // Back Face
            update_buffers(i, -j, k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color2, luminance_back);
            i += spacing;
        }
        k += spacing;
    }
}

fn render_frame<'a>(mut handle: impl Write, width: u16, height: u16, buffer: &mut [char], buffer_prev: &mut [char], cbuffer: &mut [&'a str], cbuffer_prev: &mut [&'a str], zbuffer: &mut [f32], trig_values: &[f32], spacing: f32, rotated_light_source: &Vector3f) {
    buffer_prev.copy_from_slice(buffer);
    cbuffer_prev.copy_from_slice(cbuffer);
    // buffer_prev.clone_from_slice(&buffer);
    // cbuffer_prev.clone_from_slice(&cbuffer);

    buffer.fill(' ');
    cbuffer.fill(ANSI_escape_code::color::RESET);
    zbuffer.fill(0.0);

    render_cube_axis_a(width, height, buffer, cbuffer, zbuffer, trig_values, spacing, rotated_light_source, ANSI_escape_code::color::YELLOW, ANSI_escape_code::color::WHITE);
    render_cube_axis_b(width, height, buffer, cbuffer, zbuffer, trig_values, spacing, rotated_light_source, ANSI_escape_code::color::GREEN, ANSI_escape_code::color::BLUE);
    render_cube_axis_c(width, height, buffer, cbuffer, zbuffer, trig_values, spacing, rotated_light_source, ANSI_escape_code::color::BOLD_RED, ANSI_escape_code::color::RED);

    print!("{}", ANSI_escape_code::SetCursorHome);

    let mut prev_set_color: &str = ANSI_escape_code::color::RESET;

    for (index, val) in buffer.iter().enumerate() {
        if (*val == buffer_prev[index]) && (*cbuffer[index] == *cbuffer_prev[index]) {
            continue;
        }

        let x: u32 = (index % width as usize).try_into().unwrap();
        let y: u32 = (index / width as usize).try_into().unwrap();

        // Move cursor, add color, and print char
        // printf("\x1b[%d;%dH%s%s%c", y+1, x+1, ANSI_escape_code::color::RESET, (cbuffer_iter + index)->data(), *bufferIter);
        // print!("{}{}{}", ANSI_escape_code::set_cursor_pos(y+1, x+1), cbuffer[index], val);
        if prev_set_color == cbuffer[index] {
            write!(handle, "{}{}", ANSI_escape_code::set_cursor_pos(y+1, x+1), val);
        } else {
            write!(handle, "{}{}{}", ANSI_escape_code::set_cursor_pos(y+1, x+1), cbuffer[index], val);
            prev_set_color = cbuffer[index];
        }
        // write!(handle, "{}{}{}", ANSI_escape_code::set_cursor_pos(y+1, x+1), cbuffer[index], val);
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
    println!("{}", ANSI_escape_code::EraseScreen);
    println!("{}", ANSI_escape_code::DisableAltBuffer);
    // println!("{}", ANSI_escape_code::DISABLE_ALT_BUFFER);

    println!("{}", ANSI_escape_code::color::RESET);
    println!("{}", ANSI_escape_code::CursorVisible);
    // println!("{}", ANSI_escape_code::CURSOR_VISIBLE);
}

fn main() {
    print!("{}", ANSI_escape_code::EnableAltBuffer);
    print!("{}", ANSI_escape_code::EraseScreen);
    print!("{}", ANSI_escape_code::CursorInvisible);

    let stdout = io::stdout(); // get the global stdout entity
    // optional: wrap that handle in a buffer and aquire a lock on it
    let mut handle = io::BufWriter::new(stdout.lock());

    let mut width: u16 = 100;
    let mut height: u16 = 50;

    // let term_size = get_term_size();

    let mut buffer: Vec<char> = vec![' '; (width * height).into()];
    let mut buffer_prev: Vec<char> = vec![' '; (width * height).into()];

    let mut cbuffer: Vec<&str> = vec![ANSI_escape_code::color::RESET; (width * height).into()];
    let mut cbuffer_prev: Vec<&str> = vec![ANSI_escape_code::color::RESET; (width * height).into()];

    let mut zbuffer: Vec<f32> = vec![0.0; (width * height).into()];

    let mut spacing: f32 = 3.0 / width as f32;
    let mut k1: f32 = ((width as f32) * (K2 as f32) * 3.0) / (8.0 * ((3 as f32).sqrt() * CUBE_SIZE as f32));

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

    let mut trig_values: Vec<f32> = Vec::new();

    let mut frame_times: Vec<u32> = Vec::new();

    let mut a: f32 = -std::f32::consts::FRAC_PI_2; // Axis facing the screen (z-axis)
    let mut b: f32 = -std::f32::consts::FRAC_PI_2; // Up / Down axis (y-axis)
    let mut c: f32 = std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4; // Left / Right axis (x-axis)

    let mut sin_a: f32 = a.sin();
    let mut cos_a: f32 = a.cos();

    let mut sin_b: f32 = b.sin();
    let mut cos_b: f32 = b.cos();

    let mut sin_c: f32 = c.sin();
    let mut cos_c: f32 = c.cos();
    trig_values = [sin_a, cos_a, sin_b, cos_b, sin_c, cos_c].to_vec();

    let mut d: f32 = 0.0;
    let mut e: f32 = 0.0;
    let mut f: f32 = 0.0;

    let mut sin_d: f32 = d.sin();
    let mut cos_d: f32 = d.cos();

    let mut sin_e: f32 = e.sin();
    let mut cos_e: f32 = e.cos();

    let mut sin_f: f32 = f.sin();
    let mut cos_f: f32 = f.cos();


    rotated_light_source.x = cos_d*cos_e*light_source.x + (cos_d*sin_e*sin_f - sin_d*cos_f)*light_source.y + (cos_d*sin_e*cos_f + sin_d*sin_f)*light_source.z;
    rotated_light_source.y = sin_d*cos_e*light_source.x + (sin_d*sin_e*sin_f + cos_d*cos_f)*light_source.y + (sin_d*sin_e*cos_f - cos_d*sin_f)*light_source.z;
    rotated_light_source.z = -light_source.x*sin_e + light_source.y*cos_e*sin_f + light_source.z*cos_e*cos_f;
    norm_vector(&mut rotated_light_source);
    
    // loop {
    for _i in 0..100 {
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

        render_frame(&mut handle, width, height, &mut buffer, &mut buffer_prev, &mut cbuffer, &mut cbuffer_prev, &mut zbuffer, &trig_values, spacing, &rotated_light_source);
        handle.flush().expect("Error flushing handle");

        // let sleep_dur = std::time::Duration::from_millis(200);
        // std::thread::sleep(sleep_dur);

        let us_duration = start.elapsed().as_micros();
        let ms_duration = us_duration as f64 / 1000.0;
        let fps: f64 = 1_000_000.0 / (us_duration as f64);

        print!("{}{}{}\r", ANSI_escape_code::set_cursor_pos(1, 1 + 24), ANSI_escape_code::color::RESET, ANSI_escape_code::EraseLineStartToCursor);
        print!("{fps:>7.2}fps", fps=fps);

        print!("{}{}{}\r", ANSI_escape_code::set_cursor_pos(2, 1 + 24), ANSI_escape_code::color::RESET, ANSI_escape_code::EraseLineStartToCursor);
        print!("{ms:>7.2}ms ({us:>7}us)", ms=ms_duration, us=us_duration);

        let sleep_dur = std::time::Duration::from_millis(100);
        std::thread::sleep(sleep_dur);
    }
    
    handle_exit()
}
