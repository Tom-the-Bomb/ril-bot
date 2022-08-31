//! contains all the actual image processing functions for each individual command

use ril::{prelude::*, Result};
use super::imaging::{process_gif, Frames};


/// negates the provided image
pub fn invert_func(frames: Frames) -> Result<ImageSequence<Rgba>> {
    let mut sequence =
        ImageSequence::<Rgba>::new();

    for frame in frames {
        let mut frame = frame?;
        frame.invert();

        sequence.push_frame(frame);
    }

    Ok(sequence)
}

/// rotates the hue value of the provided image by 360 degrees
pub fn huerotate_func(frames: Frames) -> Result<ImageSequence<Rgba>> {
    let mut sequence =
        ImageSequence::<Rgba>::new();

    let range = (0..360)
        .step_by(10);

    for (mut frame, deg) in process_gif(frames, range)? {
        frame.hue_rotate(deg);
        sequence.push_frame(frame);
    }

    Ok(sequence)
}