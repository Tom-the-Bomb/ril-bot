//! contains the `Error` enum used by all the self-defined functions in this module
//! such as the utility functions etc.

use std::fmt;
use serenity::{
    prelude::SerenityError,
    framework::standard::CommandError,
};

use super::helpers::humanize_bytes;


/// An error enum representing all the error types raised when resolving an image in [`ImageResolver`],
/// used by all the self-defined functions in this module such as the utility functions etc.
///
/// Implements `From<E>` for all the errors from other libraries propogated
/// and `Into<CommandError>` for easy error handling within the bot commands.
#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    /// Returned when the provided image's frame count exceeds the maximum allowed frame count
    TooManyFrames(
        /// provided frame count
        usize,
        /// maximum allowed frame count
        usize,
    ),
    /// Returned when the provided image size exceeds the maxiumum allowed size
    ImageTooLarge(
        /// provided image size
        u64,
        /// maximum allowed image size
        u64,
    ),
    /// Returned in [`super::resolver::ImageResolver::convert_emoji`] when an emoji could not be parsed from the argument
    EmojiParseError(
        /// provided argument that we failed to parse into an emoji
        String,
    ),
    /// Returned when the image URL is invalid or returned a non-ok status code
    FetchUrlError,
    /// Returned when the content-type of the provided source is not of `image/*`
    InvalidContentType,
    /// Propogated from [`reqwest::Error`]
    RequestError(
        /// Error propogated from
        reqwest::Error,
    ),
    /// Propogated from [`SerenityError`]
    SerenityError(
        /// Error propogated from
        SerenityError,
    ),
    /// Propogated from [`ril::Error`]
    RilError(
        /// Error propogated from
        ril::Error,
    ),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            match self {
                Self::TooManyFrames(count, max_frames) =>
                    format!("The provided image has a frame count of `{}` which exceeds the limit of `{}`", count, max_frames),
                Self::ImageTooLarge(size, max_size) =>
                    format!("Provided Image has a size of `{}` which exceeds the limit of `{}`",
                        humanize_bytes(*size),
                        humanize_bytes(*max_size),
                    ),
                Self::EmojiParseError(argument) =>
                    format!("An emoji could not be parsed from the provided argument: `{}`", argument),
                Self::FetchUrlError =>
                    String::from("Something went wrong during the HTTP request to the provided URL"),
                Self::InvalidContentType =>
                    String::from("Only content types of `image/*` are supported"),
                Self::RequestError(err) =>
                    format!("{}", err),
                Self::SerenityError(err) =>
                    format!("{}", err),
                Self::RilError(err) =>
                    format!("{}", err),
            }
            .as_str()
        )
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::RequestError(err)
    }
}

impl From<SerenityError> for Error {
    fn from(err: SerenityError) -> Self {
        Self::SerenityError(err)
    }
}

impl From<ril::Error> for Error {
    fn from(err: ril::Error) -> Self {
        Self::RilError(err)
    }
}

impl From<Error> for CommandError {
    fn from(err: Error) -> Self {
        Self::from(err.to_string())
    }
}