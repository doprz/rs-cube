//init function has been renamed and moved here due to its sheer size and required variables
//i plan to condense the rubix function below to rid of repetiton later. or you could try!

const CUBE_SIZE: f32 = 1.0; // Unit Cube
const FRAC_CUBE_SIZE_2: f32 = CUBE_SIZE / 2.0;
const FRAC_CUBE_SIZE_3: f32 = CUBE_SIZE / 3.0;
const GRID_SPACING: f32 = 0.04;

pub struct PointsAxisRange {
    pub a: usize,
    pub b: usize,
    pub c: usize,
}
pub struct Point3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub fn rubix(
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
