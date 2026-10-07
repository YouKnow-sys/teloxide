//! Module for serializing into `multipart/form-data`
//! ([`reqwest::multipart::Form`])
//!
//! [`reqwest::multipart::Form`]: reqwest::multipart::Form
//!
//! ## How it works
//!
//! You better not know...
//!
//! This whole module is an awful hack and we'll probably stop using it in next
//! versions (in favor of something less automatic, but more simple).

mod error;
mod serializers;

use std::future::Future;

use reqwest::multipart::Form;
use serde::Serialize;

use crate::{requests::MultipartPayload, types::InputFile};
use error::Error;
use serializers::MultipartSerializer;

/// Serializes given value into [`Form`] **taking all input files out**.
///
/// The returned future reads/opens the input files and can fail with an
/// [`io::Error`] (e.g. when reading an `InputFile::read` fails on `wasm32`,
/// where request bodies can't carry errors).
///
/// [`Form`]:  reqwest::multipart::Form
pub(crate) fn to_form<T>(val: &mut T) -> Result<impl Future<Output = Result<Form, Error>>, Error>
where
    T: Serialize + MultipartPayload,
{
    let form = val.serialize(MultipartSerializer::new())?;

    let mut files = Vec::with_capacity(1);
    val.move_files(&mut |f| files.push(f));

    Ok(attach_files(form, files))
}

/// Serializes given value into [`Form`].
///
/// The returned future reads/opens the input files and can fail with an
/// [`io::Error`] (see [`to_form`]).
///
/// [`Form`]:  reqwest::multipart::Form
pub(crate) fn to_form_ref<T: ?Sized>(
    val: &T,
) -> Result<impl Future<Output = Result<Form, Error>>, Error>
where
    T: Serialize + MultipartPayload,
{
    let form = val.serialize(MultipartSerializer::new())?;

    let mut files = Vec::with_capacity(1);
    val.copy_files(&mut |f| files.push(f));

    Ok(attach_files(form, files))
}

/// Adds a part to `form` for every file that needs to be attached.
async fn attach_files(mut form: Form, files: Vec<InputFile>) -> Result<Form, Error> {
    for file in files {
        if file.needs_attach() {
            let id = file.id().to_owned();
            form = form.part(id, file.into_part().await?);
        }
    }

    Ok(form)
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_arch = "wasm32"))]
    use tokio::fs::File;

    use super::to_form_ref;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::types::{
        InputMedia, InputMediaAnimation, InputMediaAudio, InputMediaDocument, InputMediaPhoto,
        InputMediaVideo, InputSticker, ParseMode, StickerFormat, UserId,
    };
    use crate::{
        payloads::{self, setters::*},
        types::{ChatId, InputFile, MessageEntity, MessageEntityKind},
    };

    // https://github.com/teloxide/teloxide/issues/473
    #[cfg_attr(not(target_arch = "wasm32"), tokio::test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    async fn issue_473() {
        to_form_ref(
            &payloads::SendPhoto::new(ChatId(0), InputFile::file_id("0".into())).caption_entities(
                [MessageEntity { kind: MessageEntityKind::Url, offset: 0, length: 0 }],
            ),
        )
        .unwrap()
        .await
        .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_send_media_group() {
        const CAPTION: &str = "caption";

        to_form_ref(&payloads::SendMediaGroup::new(
            ChatId(0),
            [
                InputMedia::Photo(
                    InputMediaPhoto::new(InputFile::file("../../media/teloxide-core-logo.png"))
                        .caption(CAPTION)
                        .parse_mode(ParseMode::MarkdownV2)
                        .caption_entities(entities()),
                ),
                InputMedia::Video(
                    InputMediaVideo::new(InputFile::file_id("17".into())).supports_streaming(true),
                ),
                InputMedia::Animation(
                    InputMediaAnimation::new(InputFile::read(
                        File::open("../../media/example.gif").await.unwrap(),
                    ))
                    .thumbnail(InputFile::read(
                        File::open("../../media/teloxide-core-logo.png").await.unwrap(),
                    ))
                    .duration(17),
                ),
                InputMedia::Audio(
                    InputMediaAudio::new(InputFile::url("https://example.com".parse().unwrap()))
                        .performer("a"),
                ),
                InputMedia::Document(InputMediaDocument::new(InputFile::memory(
                    &b"Hello world!"[..],
                ))),
            ],
        ))
        .unwrap()
        .await
        .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_add_sticker_to_set() {
        to_form_ref(&payloads::AddStickerToSet::new(
            UserId(0),
            "name",
            InputSticker {
                sticker: InputFile::file(
                    "../../media/
                teloxide-core-logo.png",
                ),
                emoji_list: vec!["✈️⚙️".to_owned()],
                keywords: vec![],
                mask_position: None,
                format: StickerFormat::Static,
            },
        ))
        .unwrap()
        .await
        .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_send_animation() {
        to_form_ref(
            &payloads::SendAnimation::new(
                ChatId(0),
                InputFile::file("../../media/teloxide-core-logo.png"),
            )
            .caption_entities(entities())
            .thumbnail(InputFile::read(
                File::open("../../media/teloxide-core-logo.png").await.unwrap(),
            )),
        )
        .unwrap()
        .await
        .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn entities() -> impl Iterator<Item = MessageEntity> {
        <_>::into_iter([
            MessageEntity::new(MessageEntityKind::Url, 0, 0),
            MessageEntity::new(MessageEntityKind::Pre { language: None }, 0, 0),
            MessageEntity::new(MessageEntityKind::Pre { language: Some(String::new()) }, 0, 0),
            MessageEntity::new(MessageEntityKind::Url, 0, 0),
            MessageEntity::new(
                MessageEntityKind::TextLink { url: "https://example.com".parse().unwrap() },
                0,
                0,
            ),
        ])
    }
}
