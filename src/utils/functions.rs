//! contains all the actual image processing functions for each individual command

use ril::{prelude::*, Result};

use super::imaging::Frames;

/// negates the provided image
pub fn invert_func(frames: Frames) -> Result<ImageSequence<Rgba>> {
    let mut sequence = ImageSequence::<Rgba>::new();

    for frame in frames {
        let mut frame = frame?;
        frame.invert();

        sequence.push_frame(frame);
    }

    Ok(sequence)
}