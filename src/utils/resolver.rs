//! module containing the ImageResolver struct
//! used to resolve a source image from command arguments and references

use serenity::{
    prelude::*,
    utils::ArgumentConvert,
    framework::standard::Args,
    model::{
        user::User,
        channel::Message,
        guild::{Emoji, Member},
        prelude::{
            Embed,
            StickerItem,
            Attachment,
            ChannelId,
            GuildId,
        },
    },
};

use regex::Regex;
use super::{
    Error,
    helpers::url_to_bytes,
};


lazy_static::lazy_static! {
    static ref WS_REGEX: Regex = Regex::new(r"\s+").unwrap();
}

/// the default max size for resolved images: 16 MB
pub const DEFAULT_MAX_SIZE: u64 = 16_000_000;


/// A struct for resolving a source image from command arguments or references
/// In order it try's to resolve from:
///     - A guild member from the provided argument
///     - A discord user from the provided argument
///     - A valid discord custom emoji from the provided argument
///     - A valid default emoji from the provided argument
///     if all fails or no argument was provided:
///     - checks attached files -> stickers -> embeds
///     - repeats the above for a referenced message if exists.
///     - fallbacks to command author
#[derive(Debug, Clone)]
pub struct ImageResolver {
    /// indicates the max size in bytes that we will accept for the provided image
    max_size: u64,
}

impl Default for ImageResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageResolver {
    /// returns a new instance of [`ImageResolver`] with default max size
    pub fn new() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
        }
    }

    /// a method to resolve a user inputted URL, with many checks
    pub async fn resolve_url<T: AsRef<str>>(&self, arg: T) -> Result<Vec<u8>, Error> {
        let arg = arg
            .as_ref()
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');

        let response = reqwest::get(arg)
            .await
            .map_err(|_| Error::FetchUrlError)?;

        if response.status().is_success() {
            if response.headers()
                .get("Content-Type")
                .map_or("unknown", |v| v.to_str().unwrap_or("unknown"))
                .starts_with("image/")
            {
                let content_length = response.content_length()
                    .unwrap_or(0);

                let bytes = response.bytes()
                    .await?;

                let size = content_length.max(bytes.len() as u64);
                if size >= self.max_size {
                    Err(Error::ImageTooLarge(size, self.max_size))
                } else {
                    Ok(bytes.to_vec())
                }
            } else {
                Err(Error::InvalidContentType)
            }
        } else {
            Err(Error::FetchUrlError)
        }
    }

    /// called by [`Self::get_attachments`], tries to resolve an image from message files
    async fn get_file_image(&self, attachments: &Vec<Attachment>) -> Result<Option<Vec<u8>>, Error> {
        for file in attachments {
            if file.content_type
                .clone()
                .unwrap_or_else(|| "unknown".to_string())
                .starts_with("image/")
            {
                if file.size < self.max_size {
                    let bytes = file.download().await?;

                    let size = bytes.len() as u64;
                    if size < self.max_size {
                        return Ok(Some(bytes));
                    } else {
                        return Err(Error::ImageTooLarge(size, self.max_size));
                    }
                } else {
                    return Err(Error::ImageTooLarge(file.size, self.max_size));
                }
            }
        }

        Ok(None)
    }

    /// called by [`Self::get_attachments`], tries to resolve an image from message stickers
    async fn get_sticker_image(&self, stickers: &Vec<StickerItem>) -> Result<Option<Vec<u8>>, Error> {
        for sticker in stickers {
            if let Some(url) = sticker.image_url() {
                return Ok(Some(url_to_bytes(url).await?));
            }
        }

        Ok(None)
    }

    /// called by [`Self::get_attachments`], tries to resolve an image from message embeds
    async fn get_embed_image(&self, embeds: &Vec<Embed>) -> Result<Option<Vec<u8>>, Error> {
        for embed in embeds {
            if let Some(image) = &embed.image {
                return Ok(Some(self.resolve_url(&image.url).await?));
            } else if let Some(thumbnail) = &embed.thumbnail {
                return Ok(Some(self.resolve_url(&thumbnail.url).await?));
            }
        }

        Ok(None)
    }

    /// tries to resolve attachments: (files, stickers and embeds)
    async fn get_attachments(&self, message: &Message) -> Result<Option<Vec<u8>>, Error> {
        let mut source: Option<Vec<u8>> = None;

        if !message.attachments.is_empty() {
            source = self.get_file_image(&message.attachments).await?;
        }

        if source.is_none() && !message.sticker_items.is_empty() {
            source = self.get_sticker_image(&message.sticker_items).await?;
        }

        if source.is_none() && !message.embeds.is_empty() {
            source = self.get_embed_image(&message.embeds).await?;
        }

        Ok(source)
    }

    /// run's conversions on the argument and referenced message's content
    pub async fn try_conversions(
        &self,
        ctx: &Context,
        guild: Option<GuildId>,
        channel: Option<ChannelId>,
        arg: &str,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(if let Ok(out) =
            Member::convert(ctx, guild, channel, &*arg)
            .await
        {
            Some(url_to_bytes(out.face())
                .await?)
        } else if let Ok(out) =
            User::convert(ctx, guild, channel, &*arg)
            .await
        {
            Some(url_to_bytes(out.face())
                .await?)
        } else if let Ok(out) =
            Emoji::convert(ctx, guild, channel, &*arg)
            .await
        {
            Some(url_to_bytes(out.url())
                .await?)
        } else if let Ok(out) =
            url_to_bytes(format!("https://emojicdn.elk.sh/{arg}?style=twitter"))
                .await
        {
            Some(out)
        } else if let Ok(out) =
            match self.resolve_url(arg)
                .await
            {
                Err(err @ Error::ImageTooLarge(..)) => return Err(err),
                other => other,
            }
        {
            Some(out)
        } else {
            None
        })
    }

    /// the primary method to call to resolve an image from the provided `context`, `message` and `args`
    pub async fn resolve(&self, ctx: &Context, message: &Message, args: &mut Args) -> Result<Vec<u8>, Error> {
        let arg = args.single_quoted::<String>().ok();

        if let Some(arg) = arg {
            if let Some(bytes) =
                self.try_conversions(
                    ctx,
                    message.guild_id,
                    Some(message.channel_id),
                    &*arg,
                )
                .await
                .transpose()
            {
                return bytes;
            }
        }

        if let Some(bytes) =
            self.get_attachments(message)
            .await?
        {
            return Ok(bytes);
        }

        if let Some(referenced) = &message.referenced_message {
            if let Some(bytes) =
                self.get_attachments(referenced)
                .await?
            {
                return Ok(bytes);
            }

            if referenced.content.len() > 0 {
                let content = WS_REGEX
                    .split(referenced.content.as_str())
                    .next();

                if let Some(content) = content {
                    if let Some(bytes) = self.try_conversions(
                            ctx,
                            referenced.guild_id,
                            Some(referenced.channel_id),
                            content,
                        )
                        .await
                        .transpose()
                    {
                        return bytes;
                    }
                }
            }
        }

        Ok(url_to_bytes(message.author.face()).await?)
    }
}