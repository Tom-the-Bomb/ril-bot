//! Contains general utility functions for image processing

use std::{
    vec::IntoIter,
    iter::{Zip, Cycle},
    time::Instant,
    borrow::Cow,
};

use serenity::{
    prelude::*,
    framework::standard::CommandResult,
    model::prelude::{Message, AttachmentType},
};

use ril::prelude::*;
use super::{
    Error,
    functions::contain_size,
};

/// TypeAlias for an imagesequence the bot decodes into and passes around
pub type Frames = ImageSequence<Rgba>;

/// constant representing the default max dimensions for an input image
pub const DEFAULT_MAX_DIM: u32 = 500;
/// constant representing the default max frame count for an input image
pub const DEFAULT_MAX_FRAMES: usize = 200;


/// a helper function to send the output image to the discord channel,
/// used by [`ImageExecutor::run`]
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
    let content = format!("**Process Time:** `{elapsed} ms`");
    let format = if is_gif { "gif" } else { "png" };

    message.channel_id.send_message(ctx,
        |msg| {
            msg.content(content)
                .reference_message(message)
                .allowed_mentions(|am| am.empty_parse())
                .add_file(
                    AttachmentType::Bytes {
                        data: output.into(),
                        filename: format!("output.{format}"),
                    }
                )
        }
    ).await?;

    Ok(())
}

/// a wrapper struct to allow for a dynamic amount of arguments
/// passed to the image function being executed
#[derive(Clone)]
pub struct ImageArguments<A = ()> {
    /// the input image
    pub frames: Frames,
    /// a vector of arguments
    pub arguments: Vec<A>,
}

/// a general struct to execute a function to process an image
/// and hold configuration information for the execution
///
/// does repetitive things such as resolving, opening, encoding and sending the image.
#[derive(Clone)]
pub struct ImageExecutor<'a, F, A>
where
    F: Fn(ImageArguments<A>) -> ril::Result<Frames> + Send + Sync + 'static,
{
    /// the current command context
    ctx: &'a Context,
    /// the invokation message of the command
    message: &'a Message,
    /// the image function to execute
    function: Option<F>,
    /// the maximum width allowed for an image
    max_width: Option<u32>,
    /// the maximum height allowed for an image
    max_height: Option<u32>,
    /// the maximum number of frames allowed for an image
    max_frames: Option<usize>,
    /// any extra arguments passed to the function
    arguments: Vec<A>,
}

impl<'a, F, A> ImageExecutor<'a, F, A>
where
    A: Send + Sync + 'static,
    F: Fn(ImageArguments<A>) -> ril::Result<Frames> + Send + Sync + 'static,
{
    /// creates a new instance of [`ImageExecutor`] with the basic, required information passed
    #[must_use]
    pub const fn new(ctx: &'a Context, message: &'a Message) -> Self {
        Self {
            ctx, message,
            function: None,
            max_width: None,
            max_height: Some(DEFAULT_MAX_DIM),
            max_frames: Some(DEFAULT_MAX_FRAMES),
            arguments: Vec::new(),
        }
    }

    /// a builder method to set the image function to execute, must be called
    #[must_use]
    pub fn function(mut self, function: F) -> Self {
        self.function = Some(function);
        self
    }

    /// a builder method to set [`self.max_width`]
    #[must_use]
    #[allow(dead_code)]
    pub const fn max_width(mut self, max_width: u32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// a builder method to set [`self.max_height`]
    #[must_use]
    #[allow(dead_code)]
    pub const fn max_height(mut self, max_height: u32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// a builder method to set [`self.max_frames`]
    #[must_use]
    #[allow(dead_code)]
    pub const fn max_frames(mut self, max_frames: usize) -> Self {
        self.max_frames = Some(max_frames);
        self
    }

    /// a builder method to pass in arguments to the image function
    #[must_use]
    #[allow(dead_code)]
    pub fn arguments(mut self, arguments: Vec<A>) -> Self {
        self.arguments = arguments;
        self
    }

    /// the primary method to call, this basically uses all of the passed information
    /// it accepts the bytes of the source image which you should resolve beforehand using `ImageResolver`
    /// and proceeds to execute the provided function, with all the wrapping tasks also done here
    pub async fn run(self, bytes: Vec<u8>) -> CommandResult {
        let instant = Instant::now();
        let (result, is_gif) = tokio::task::spawn_blocking(
            move || -> Result<(Vec<u8>, bool), Error> {
                let mut image = ImageSequence::<Rgba>::from_bytes_inferred(&bytes[..])?
                    .into_sequence()?;

                let max_frames = self.max_frames
                    .unwrap_or(DEFAULT_MAX_FRAMES);

                if image.len() > max_frames {
                    return Err(Error::TooManyFrames(image.len(), max_frames))
                }

                image = contain_size(
                    ImageArguments { frames: image, arguments: Vec::new() },
                    self.max_width,
                    self.max_height,
                )?;

                let sequence = self.function
                    .expect("No function was specified or passed, have you called the builder method `function(f)`?")
                    (ImageArguments::<A> { frames: image, arguments: self.arguments })?
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

        send_output(
            self.ctx,
            self.message,
            result, elapsed, is_gif,
        )
            .await?;

        Ok(())
    }
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