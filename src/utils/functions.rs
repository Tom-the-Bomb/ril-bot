use ril::{prelude::*, Result};

use super::imaging::Frames;

pub fn invert_func(frames: Frames) -> Result<ImageSequence<Rgba>> {
    let mut sequence = ImageSequence::<Rgba>::new();

    for frame in frames {
        let mut frame = frame?;
        frame.invert();

        sequence.push_frame(frame);
    }

    Ok(sequence)
}