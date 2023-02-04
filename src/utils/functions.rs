//! contains all the actual image processing functions for each individual command

#![allow(clippy::unnecessary_wraps)]

use ril::{prelude::*, Result};
use super::imaging::{process_gif, Frames, ImageArguments};

lazy_static::lazy_static! {
    static ref IMPACT_FONT: Font = Font::open("./assets/impact.ttf", 30.0).unwrap();
}


/// negates the provided image
pub fn invert_func(data: ImageArguments) -> Result<Frames> {
    let mut sequence =
        ImageSequence::<Rgba>::new();

    for frame in data.frames {
        let delay = frame.delay();
        let disposal = frame.disposal();

        let frame = Frame::from_image(
            frame.into_image()
                .map_rgb_pixels(|px| px.inverted())
        )
        .with_delay(delay)
        .with_disposal(disposal);

        sequence.push_frame(frame);
    }

    Ok(sequence)
}

/// rotates the hue value of the provided image by 360 degrees
pub fn huerotate_func(data: ImageArguments) -> Result<Frames> {
    let mut sequence =
        ImageSequence::<Rgba>::new();

    let range = (0..360)
        .step_by(10);

    for (mut frame, deg) in process_gif(data.frames, range) {
        frame.hue_rotate(deg);
        sequence.push_frame(frame);
    }

    Ok(sequence)
}

/// adds a meme caption onto a provided image
pub fn caption_func(data: ImageArguments<String>) -> Result<Frames> {
    let mut sequence =
        ImageSequence::<Rgba>::new();
    let segment = TextSegment::new(
        &IMPACT_FONT, data.arguments[0].as_str(), Rgba::black()
    );

    for frame in data.frames {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let mut layout = TextLayout::new()
            .with_width((f64::from(frame.width()) * 0.9) as u32)
            .with_wrap(WrapStyle::Word)
            .centered()
            .with_segment(&segment);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let extra_height =
            (f64::from(layout.height()) / 9.0 * 10.0) as u32;

        layout = layout
            .with_position(frame.width() / 2, extra_height / 2)
            .with_segment(&segment);

        let mut image = Image::<Rgba>::new(
            frame.width(),
            frame.height() + extra_height,
            Rgba::white(),
        );
        image.draw(&layout);
        image.paste(0, extra_height, frame.image());

        let frame = Frame::from_image(image)
            .with_delay(frame.delay())
            .with_disposal(frame.disposal());
        sequence.push_frame(frame);
    }

    Ok(sequence)
}

/// resizes an image to a provided size, only if it is larger
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn contain_size(
    data: ImageArguments<()>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<Frames> {
    let frames = data.frames;

    if width.is_none() && height.is_none() {
        return Ok(frames);
    }

    let (w, h) = if let Some(first) =
        frames.first_frame()
    {
        (f64::from(first.width()), f64::from(first.height()))
    } else {
        return Ok(frames);
    };

    let resolved_width = width
        .unwrap_or_else(|| {
            height.map_or(
                w as u32,
                |height| {
                    ((f64::from(height) / h) * w).ceil() as u32
                }
            )
        });

    let resolved_height = height
        .unwrap_or_else(|| {
            width.map_or(
                h as u32,
                |width| {
                    ((f64::from(width) / w) * h).ceil() as u32
                }
            )
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