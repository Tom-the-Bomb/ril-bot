//! module containing the [`ImageResolver`] struct
//! used to resolve a source image from command arguments and references

use serenity::{
    prelude::*,
    utils::ArgumentConvert,
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

use regex::{Regex, RegexBuilder};
use crate::ClientData;
use super::{
    Error,
    helpers::url_to_bytes,
};


lazy_static::lazy_static! {
    /// regex for removing whitespace in a string
    static ref WS_REGEX: Regex = Regex::new(r"\s+").unwrap();
    /// regex that matches a discord emoji
    static ref EMOJI_REGEX: Regex = Regex::new(r"^<(a?):([a-zA-Z0-9_]{1,32}):([0-9]{15,20})>$").unwrap();
    /// regex that matches a discord snowflake (id)
    static ref ID_REGEX: Regex = Regex::new(r"^([0-9]{15,20})$").unwrap();
    /// regex that matches a tenor page url
    static ref TENOR_PAGE_REGEX: Regex = RegexBuilder::new(r"^https?://(www\.)?tenor\.com/view/\S+/?$")
        .case_insensitive(true)
        .build()
        .unwrap();
    /// regex that matches a tenor asset url
    static ref TENOR_ASSET_URL: Regex = RegexBuilder::new(r"https?://(www\.)?c\.tenor\.com/\S+/\S+\.gif/?")
        .case_insensitive(true)
        .build()
        .unwrap();
    /// regex that matches an imgur page url
    static ref IMGUR_PAGE_REGEX: Regex = RegexBuilder::new(r"^https?://(www\.)?imgur.com/(\S+)/?$")
        .case_insensitive(true)
        .build()
        .unwrap();
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
#[allow(clippy::module_name_repetitions)]
pub struct ImageResolver {
    /// indicates the max size in bytes that we will accept for the provided image
    pub max_size: u64,
    /// indicates whether or not the image was resolved from an argument
    pub arg_resolved: bool,
}

impl Default for ImageResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::module_name_repetitions)]
impl ImageResolver {
    /// returns a new instance of [`ImageResolver`] with default max size
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            arg_resolved: true,
        }
    }

    /// a method to resolve a user inputted URL, with many checks
    pub async fn resolve_url<T>(&self, client: Option<&reqwest::Client>, arg: T) -> Result<Vec<u8>, Error>
    where
        T: AsRef<str> + Send
    {
        let arg = arg
            .as_ref()
            .trim_start_matches('<')
            .trim_end_matches('>')
            .trim();

        let response = if let Some(client) = client {
            client.get(arg)
                .send()
                .await
        } else {
            reqwest::get(arg)
                .await
        }
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
            } else if TENOR_PAGE_REGEX.is_match(arg) {
                let asset = TENOR_ASSET_URL.find(response.text().await?.as_str())
                    .map(|mat| mat.as_str().to_string())
                    .ok_or(Error::InvalidContentType)?;

                url_to_bytes(client, asset)
                    .await
            } else if let Some(captures) =
                IMGUR_PAGE_REGEX.captures(arg)
            {
                let imgur_id = captures.get(2)
                    .ok_or(Error::InvalidContentType)?
                    .as_str();

                url_to_bytes(client, format!("https://i.imgur.com/{imgur_id}.gif"))
                    .await
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
                    }
                    return Err(
                        Error::ImageTooLarge(size, self.max_size)
                    );
                }
                return Err(
                    Error::ImageTooLarge(file.size, self.max_size)
                );
            }
        }

        Ok(None)
    }

    /// called by [`Self::get_attachments`], tries to resolve an image from message stickers
    async fn get_sticker_image(
        &self,
        client: Option<&reqwest::Client>,
        stickers: &Vec<StickerItem>,
    ) -> Result<Option<Vec<u8>>, Error> {
        for sticker in stickers {
            if let Some(url) = sticker.image_url() {
                return Ok(Some(url_to_bytes(client, url).await?));
            }
        }

        Ok(None)
    }

    /// called by [`Self::get_attachments`], tries to resolve an image from message embeds
    async fn get_embed_image(&self,
        client: Option<&reqwest::Client>,
        embeds: &Vec<Embed>,
    ) -> Result<Option<Vec<u8>>, Error> {
        for embed in embeds {
            if let Some(image) = &embed.image {
                return Ok(Some(self.resolve_url(client, &image.url).await?));
            } else if let Some(thumbnail) = &embed.thumbnail {
                return Ok(Some(self.resolve_url(client, &thumbnail.url).await?));
            }
        }

        Ok(None)
    }

    /// tries to resolve attachments: (files, stickers and embeds)
    #[allow(clippy::useless_let_if_seq)]
    async fn get_attachments(
        &self,
        client: Option<&reqwest::Client>,
        message: &Message,
    ) -> Result<Option<Vec<u8>>, Error> {
        let mut source: Option<Vec<u8>> = None;

        if !message.attachments.is_empty() {
            source = self.get_file_image(&message.attachments).await?;
        }

        if source.is_none() && !message.sticker_items.is_empty() {
            source = self.get_sticker_image(client,&message.sticker_items).await?;
        }

        if source.is_none() && !message.embeds.is_empty() {
            source = self.get_embed_image(client, &message.embeds).await?;
        }

        Ok(source)
    }

    /// fetches the member's face but fallbacks to `png` format instead of `webp`
    #[must_use]
    fn member_avatar_url(member: &Member) -> String {
        let is_gif = member.avatar.as_ref()
            .or(member.user.avatar.as_ref())
            .map_or(false, |av| av.starts_with("a_"));

        member.face()
            .replace(".webp", if is_gif { ".gif" } else { ".png" })
    }

    /// fetches the user's face but fallbacks to `png` format instead of `webp`
    #[must_use]
    fn user_avatar_url(user: &User) -> String {
        let is_gif = user.avatar.as_ref()
            .map_or(false, |av| av.starts_with("a_"));

        user.face()
            .replace(".webp", if is_gif { ".gif" } else { ".png" })
    }

    /// a method to fetch the emoji image from a `<:name:id>` formatted emoji or simply an `id`
    #[allow(clippy::option_if_let_else)]
    pub async fn convert_emoji(client: Option<&reqwest::Client>, argument: &str) -> Result<Vec<u8>, Error> {
        let (animated, id) =
            if let Some(captures) = EMOJI_REGEX.captures(argument)
        {
            (
                captures.get(1)
                    .is_some(),
                captures.get(3)
                    .map(|id| id.as_str().to_string()),
            )
        } else if let Some(mat) = ID_REGEX.find(argument) {
            (false, Some(mat.as_str().to_string()))
        } else {
            (false, None)
        };

        let id = id.ok_or_else(||
            Error::EmojiParseError(argument.to_string())
        )?;

        let fmt = if animated { "gif" } else { "png" };
        let url = format!("https://cdn.discordapp.com/emojis/{id}.{fmt}");

        url_to_bytes(client, url)
            .await
    }

    /// run's conversions on the argument and referenced message's content
    pub async fn try_conversions(
        &self,
        client: Option<&reqwest::Client>,
        ctx: &Context,
        guild: Option<GuildId>,
        channel: Option<ChannelId>,
        arg: &str,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(if let Ok(out) =
            Member::convert(ctx, guild, channel, arg)
                .await
        {
            Some(url_to_bytes(client, Self::member_avatar_url(&out))
                .await?)
        } else if let Ok(out) =
            User::convert(ctx, guild, channel, arg)
                .await
        {
            Some(url_to_bytes(client, Self::user_avatar_url(&out))
                .await?)
        } else if let Ok(out) =
            Emoji::convert(ctx, guild, channel, arg)
                .await
        {
            Some(url_to_bytes(client, out.url())
                .await?)
        } else if let Ok(out) =
            Self::convert_emoji(client, arg)
                .await
        {
            Some(out)
        } else if let Ok(out) =
            url_to_bytes(client, format!("https://emojicdn.elk.sh/{arg}?style=twitter"))
                .await
        {
            Some(out)
        } else if let Ok(out) =
            match self.resolve_url(client, arg)
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
    pub async fn resolve(&mut self, ctx: &Context, message: &Message, arg: Option<String>) -> Result<Vec<u8>, Error> {
        let client_data = ctx.data.read()
            .await;

        let client = client_data
            .get::<ClientData>();

        if let Some(arg) = arg {
            if let Some(bytes) =
                self.try_conversions(
                    client,
                    ctx,
                    message.guild_id,
                    Some(message.channel_id),
                    arg.as_str(),
                )
                .await
                .transpose()
            {
                return bytes;
            }

            self.arg_resolved = false;
        }

        if let Some(bytes) =
            self.get_attachments(client, message)
            .await?
        {
            return Ok(bytes);
        }

        if let Some(referenced) = &message.referenced_message {
            if let Some(bytes) =
                self.get_attachments(client, referenced)
                .await?
            {
                return Ok(bytes);
            }

            if !referenced.content.is_empty() {
                let content = WS_REGEX
                    .split(referenced.content.as_str())
                    .next();

                if let Some(content) = content {
                    if let Some(bytes) = self.try_conversions(
                            client,
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

        let avatar = if let Some(guild) = message.guild_id {
            Self::member_avatar_url(
                &guild.member(ctx, message.author.id)
                    .await?
            )
        } else {
            Self::user_avatar_url(&message.author)
        };

        let fallback = url_to_bytes(client, avatar)
            .await?;

        Ok(fallback)
    }
}