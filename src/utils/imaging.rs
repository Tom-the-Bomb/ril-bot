use serenity::{
    Result,
    prelude::*,
    framework::standard::{Args, CommandResult},
    model::prelude::{Message, AttachmentType},
};

use ril::prelude::*;

use std::{time::Instant, borrow::Cow};
use super::{Error, ImageResolver};

pub type Frames<'a> = DynamicFrameIterator<Rgba, &'a [u8]>;

pub async fn send_output<'a, T>(
    ctx: &Context,
    message: &Message,
    output: T,
    elapsed: u128,
    is_gif: bool,
) -> Result<()>
where
    T: Into<Cow<'a, [u8]>>
{
    let content = format!("**Process Time:** `{} ms`", elapsed);
    let format = if is_gif { "gif" } else { "png" };

    message.channel_id.send_message(ctx,
        |msg| {
            msg.content(content)
                .reference_message(message)
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

pub async fn do_command<F>(
    ctx: &Context,
    message: &Message,
    mut args: Args,
    function: F,
) -> CommandResult
where
    F: Fn(Frames) -> ril::Result<ImageSequence<Rgba>> + Send + Sync + 'static,
{
    let resolved = ImageResolver::new()
        .resolve(ctx, message, &mut args)
        .await?;

    let instant = Instant::now();
    let (result, is_gif) = tokio::task::spawn_blocking(
        move || -> ril::Result<(Vec<u8>, bool)> {
            let image = ImageSequence::<Rgba>::from_bytes_inferred(&resolved[..])?;

            let sequence = function(image)?;
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
    .map_err(|e| Error::from(e))?;

    let elapsed = instant.elapsed().as_millis();
    send_output(ctx, message, result, elapsed, is_gif)
        .await?;

    Ok(())
}