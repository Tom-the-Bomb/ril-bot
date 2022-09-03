//! Contains general utility functions for image processing

use std::{
    vec::IntoIter,
    iter::{Zip, Cycle},
    time::Instant,
    borrow::Cow
};

use serenity::{
    prelude::*,
    framework::standard::{Args, CommandResult},
    model::prelude::{Message, AttachmentType},
};

use ril::prelude::*;
use super::{
    Error,
    ImageResolver,
    functions::contain_size,
};

pub type Frames = ImageSequence<Rgba>;


/// a helper function to send the output image to the discord channel,
/// used by [`do_command`]
pub async fn send_output<'a, T>(
    ctx: &Context,
    message: &Message,
    output: T,
    elapsed: u128,
    is_gif: bool,
) -> serenity::Result<()>
where
    T: Into<Cow<'a, [u8]>>
{
    let content = format!("**Process Time:** `{} ms`", elapsed);
    let format = if is_gif { "gif" } else { "png" };

    message.channel_id.send_message(ctx,
        |msg| {
            msg.content(content)
                .reference_message(message)
                .allowed_mentions(|am| am.empty_parse())
                .add_file(
                    AttachmentType::Bytes {
                        data: output.into(),
                        filename: format!("output.{}", format),
                    }
                )
        }
    ).await?;

    Ok(())
}

/// a general utility function to execute a function to process an image
///
/// does repetitive things such as resolving, opening, encoding and sending the image.
pub async fn do_command<F>(
    ctx: &Context,
    message: &Message,
    mut args: Args,
    function: F,
    max_size: (Option<u32>, Option<u32>),
) -> CommandResult
where
    F: Fn(Frames) -> ril::Result<Frames> + Send + Sync + 'static,
{
    let resolved = ImageResolver::new()
        .resolve(ctx, message, &mut args)
        .await?;

    let instant = Instant::now();
    let (result, is_gif) = tokio::task::spawn_blocking(
        move || -> ril::Result<(Vec<u8>, bool)> {
            let mut image = ImageSequence::<Rgba>::from_bytes_inferred(&resolved[..])?
                .into_sequence()?;

            let (width, height) = max_size;
            image = contain_size(image, width, height)?;

            let sequence = function(image)?
                .looped_infinitely();

            let is_gif = sequence.len() > 1;
            let format =
                if is_gif {
                    ImageFormat::Gif
                } else {
                    ImageFormat::Png
                };

            let mut bytes: Vec<u8> = Vec::new();
            sequence.encode(format, &mut bytes)?;

            Ok((bytes, is_gif))
        }
    )
    .await?
    .map_err(Error::from)?;

    let elapsed = instant.elapsed()
        .as_millis();

    send_output(ctx, message, result, elapsed, is_gif)
        .await?;

    Ok(())
}

/// helper function that zips together an iterator that generates a gif
/// with the original input gif frames to allow for partial gif support on gif functions
pub fn process_gif<I>(frames: Frames, iterable: I)
    -> Zip<Cycle<IntoIter<Frame<Rgba>>>, I>
where
    I: Iterator<Item = i32>
{
    frames
        .into_iter()
        .cycle()
        .zip(iterable)
}