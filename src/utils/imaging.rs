use serenity::{
    Result,
    prelude::*,
    framework::standard::{Args, CommandResult},
    model::prelude::{Message, AttachmentType},
};

use ril::prelude::*;

use std::borrow::Cow;
use super::{Error, ImageResolver};

pub type Frames<'a> = DynamicFrameIterator<Rgba, &'a [u8]>;

pub async fn send_output<'a, T>(
    ctx: &Context,
    message: &Message,
    output: T,
    is_gif: bool,
) -> Result<()>
    where T: Into<Cow<'a, [u8]>>
{
    let format = if is_gif { "gif" } else { "png" };

    message.channel_id.send_message(ctx,
        |msg| {
            msg.reference_message(message)
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

    send_output(
        ctx,
        message,
        result,
        is_gif,
    )
    .await?;

    Ok(())
}