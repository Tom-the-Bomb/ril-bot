//! contains all the actual image processing functions for each individual command

#![allow(clippy::unnecessary_wraps)]

use ril::{prelude::*, Result};
use super::imaging::{process_gif, Frames};


/// negates the provided image
pub fn invert_func(frames: Frames) -> Result<Frames> {
    let mut sequence =
        ImageSequence::<Rgba>::new();

    for mut frame in frames {
        frame.invert();
        sequence.push_frame(frame);
    }

    Ok(sequence)
}

/// rotates the hue value of the provided image by 360 degrees
pub fn huerotate_func(frames: Frames) -> Result<Frames> {
    let mut sequence =
        ImageSequence::<Rgba>::new();

    let range = (0..360)
        .step_by(10);

    for (mut frame, deg) in process_gif(frames, range) {
        frame.hue_rotate(deg);
        sequence.push_frame(frame);
    }

    Ok(sequence)
}

/// resizes an image to a provided size, only if it is larger
pub fn contain_size(
    frames: Frames,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Frames> {
    if width.is_none() && height.is_none() {
        return Ok(frames);
    }

    let first = frames.first_frame();

    let w = first.width() as f32;
    let h = first.height() as f32;

    let resolved_width = width
        .unwrap_or_else(|| {
            if let Some(height) = height {
                ((height as f32 / h) * w).ceil() as u32
            } else {
                w as u32
            }
        });

    let resolved_height = height
        .unwrap_or_else(|| {
            if let Some(width) = width {
                ((width as f32 / w) * h).ceil() as u32
            } else {
                h as u32
            }
        });

    if w as u32 >= resolved_width || h as u32 >= resolved_height {
        let mut sequence =
            ImageSequence::<Rgba>::new();

        for mut frame in frames {
            frame.resize(
                resolved_width,
                resolved_height,
                ResizeAlgorithm::Lanczos3,
            );
            sequence.push_frame(frame);
        }

        Ok(sequence)
    } else {
        Ok(frames)
    }
}