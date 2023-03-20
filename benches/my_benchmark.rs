use criterion::{black_box, criterion_group, criterion_main, Criterion};

const CUBE_SIZE: f32 = 1.0; // Unit Cube
const FRAC_CUBE_SIZE_2: f32 = CUBE_SIZE / 2.0;
const FRAC_CUBE_SIZE_3: f32 = CUBE_SIZE / 3.0;
const FRAC_CUBE_SIZE_4: f32 = CUBE_SIZE / 4.0;

const GRID_SPACING: f32 = 0.04;
const GRID_LINE_COLOR: &str = "\x1B[30m";

const K2: f32 = 10.0;

struct Vector3f {
    x: f32,
    y: f32,
    z: f32,
}

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

fn render_cube_axis(trig_values: &[f32], spacing: f32, rotated_light_source: &Vector3f, color1: &str, color2: &str) {
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
    let k: f32 = FRAC_CUBE_SIZE_2;

    // y
    let mut i: f32 = -FRAC_CUBE_SIZE_2;
    while i <= FRAC_CUBE_SIZE_2 {
        // x
        let mut j: f32 = -FRAC_CUBE_SIZE_2;
        while j <= FRAC_CUBE_SIZE_2 {
            let mut char_color1: &str = &color1;
            let mut char_color2: &str = &color2;
            if (i > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                    i < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (i > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                    i < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (j > (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                    j < (-FRAC_CUBE_SIZE_2 + FRAC_CUBE_SIZE_3) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            } else if (j > (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) - GRID_SPACING &&
                    j < (FRAC_CUBE_SIZE_2 - FRAC_CUBE_SIZE_3) + GRID_SPACING) {
                char_color1 = GRID_LINE_COLOR;
                char_color2 = GRID_LINE_COLOR;
            }

            // Front Face
            // update_buffers(i, j, k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color1, luminance_front);

            // Back Face
            // update_buffers(i, j, -k, width, height, buffer, zbuffer, cbuffer, trig_values, char_color2, luminance_back);

            j += spacing;
        }
        i += spacing;
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let width: u16 = 100;
    let height: u16 = 50;

    let spacing: f32 = black_box(3.0 / width as f32);
    let k1: f32 = ((width as f32) * (K2 as f32) * 3.0) / (8.0 * ((3 as f32).sqrt() * CUBE_SIZE as f32));

    let a_local: f32 = -std::f32::consts::FRAC_PI_2; // Axis facing the screen (z-axis)
    let b_local: f32 = -std::f32::consts::FRAC_PI_2; // Up / Down axis (y-axis)
    let c_local: f32 = std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4; // Left / Right axis (x-axis)

    let sin_a: f32 = a_local.sin();
    let cos_a: f32 = a_local.cos();

    let sin_b: f32 = b_local.sin();
    let cos_b: f32 = b_local.cos();

    let sin_c: f32 = c_local.sin();
    let cos_c: f32 = c_local.cos();
    let trig_values: Vec<f32> = black_box(vec![sin_a, cos_a, sin_b, cos_b, sin_c, cos_c]);

    let d: f32 = 0.0;
    let e: f32 = 0.0;
    let f: f32 = 0.0;

    let sin_d: f32 = d.sin();
    let cos_d: f32 = d.cos();

    let sin_e: f32 = e.sin();
    let cos_e: f32 = e.cos();

    let sin_f: f32 = f.sin();
    let cos_f: f32 = f.cos();

    let light_source = Vector3f {
        x: 0.0, 
        y: 1.0, 
        z: -1.0
    };
    let mut rotated_light_source = black_box(
        Vector3f {
            x: cos_d*cos_e*light_source.x + (cos_d*sin_e*sin_f - sin_d*cos_f)*light_source.y + (cos_d*sin_e*cos_f + sin_d*sin_f)*light_source.z,
            y: sin_d*cos_e*light_source.x + (sin_d*sin_e*sin_f + cos_d*cos_f)*light_source.y + (sin_d*sin_e*cos_f - cos_d*sin_f)*light_source.z,
            z: -light_source.x*sin_e + light_source.y*cos_e*sin_f + light_source.z*cos_e*cos_f,
        }
    );
    norm_vector(&mut rotated_light_source);
    
    c.bench_function(
        "render cube axis", |b| b.iter(|| render_cube_axis(&trig_values, spacing, &rotated_light_source, GRID_LINE_COLOR, GRID_LINE_COLOR))
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
