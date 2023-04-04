// Copyright (c) 2023 doprz
// SPDX-License-Identifier: MIT OR Apache-2.0

use rs_cube::run;

fn main() {
    pollster::block_on(run());
}
