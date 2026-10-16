//! Image post-processing filters translated from ands/lightmapper:
//! - UV Seam Dilation (`lmImageDilate`)
//! - Non-zero Box Smoothing (`lmImageSmooth`)

/// Dilates non-zero pixels into neighboring empty/zero pixels by 1 pixel.
/// This eliminates black edge fringe artifacts across UV island seams.
pub fn dilate_lightmap(image: &[f32], width: usize, height: usize, channels: usize) -> Vec<f32> {
    let mut result = image.to_vec();
    let offsets = [
        (-1, -1), (0, -1), (1, -1),
        (-1,  0),          (1,  0),
        (-1,  1), (0,  1), (1,  1),
    ];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * channels;
            // Check if current pixel is empty (all channels zero)
            let is_zero = (0..channels).all(|c| image[idx + c] == 0.0);
            if !is_zero {
                continue;
            }

            // Accumulate valid neighbors
            let mut sum = vec![0.0f32; channels];
            let mut count = 0;

            for (dx, dy) in offsets {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                    let n_idx = (ny as usize * width + nx as usize) * channels;
                    let n_has_val = (0..channels).any(|c| image[n_idx + c] > 0.0);
                    if n_has_val {
                        for c in 0..channels {
                            sum[c] += image[n_idx + c];
                        }
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let inv = 1.0 / count as f32;
                for c in 0..channels {
                    result[idx + c] = sum[c] * inv;
                }
            }
        }
    }

    result
}

/// Applies repeated dilation passes to bleed colors beyond UV seams.
pub fn multi_pass_dilate(
    mut image: Vec<f32>,
    width: usize,
    height: usize,
    channels: usize,
    passes: u32,
) -> Vec<f32> {
    for _ in 0..passes {
        image = dilate_lightmap(&image, width, height, channels);
    }
    image
}

/// Applies a 3x3 box blur filter on populated (non-zero) lightmap pixels
/// to smooth out Monte Carlo sampling variance without bleeding into empty space.
pub fn smooth_lightmap(image: &[f32], width: usize, height: usize, channels: usize) -> Vec<f32> {
    let mut result = image.to_vec();
    let offsets = [
        (-1, -1), (0, -1), (1, -1),
        (-1,  0), (0,  0), (1,  0),
        (-1,  1), (0,  1), (1,  1),
    ];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * channels;
            let is_zero = (0..channels).all(|c| image[idx + c] == 0.0);
            if is_zero {
                continue;
            }

            let mut sum = vec![0.0f32; channels];
            let mut count = 0;

            for (dx, dy) in offsets {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx >= 0 && nx < width as isize && ny >= 0 && ny < height as isize {
                    let n_idx = (ny as usize * width + nx as usize) * channels;
                    let n_has_val = (0..channels).any(|c| image[n_idx + c] > 0.0);
                    if n_has_val {
                        for c in 0..channels {
                            sum[c] += image[n_idx + c];
                        }
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let inv = 1.0 / count as f32;
                for c in 0..channels {
                    result[idx + c] = sum[c] * inv;
                }
            }
        }
    }

    result
}
