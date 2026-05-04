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

use crate::requests::MultipartPayload;
use error::Error;
use serializers::MultipartSerializer;

/// Serializes given value into [`Form`] **taking all input files out**.
///
/// [`Form`]:  reqwest::multipart::Form
pub(crate) fn to_form<T>(val: &mut T) -> Result<impl Future<Output = Form>, Error>
where
    T: Serialize + MultipartPayload,
{
    let direct_upload_fields = collect_direct_upload_fields(val);
    let direct_file_fields = direct_upload_fields.iter().map(|(_, field)| *field).collect();

    let mut form = val.serialize(MultipartSerializer::new(direct_file_fields))?;

    let mut vec = Vec::with_capacity(1);
    val.move_files(&mut |f| vec.push(f));
    let iter = vec.into_iter();

    let fut = async move {
        for file in iter {
            if file.needs_attach() {
                let id = file.id().to_owned();
                let field = direct_upload_fields
                    .iter()
                    .find_map(|(direct_id, field)| (direct_id == &id).then_some(*field))
                    .unwrap_or(&id);
                if let Some(part) = file.into_part(field.to_owned()) {
                    form = form.part(field.to_owned(), part.await);
                }
            }
        }

        form
    };

    Ok(fut)
}

/// Serializes given value into [`Form`].
///
/// [`Form`]:  reqwest::multipart::Form
pub(crate) fn to_form_ref<T: ?Sized>(val: &T) -> Result<impl Future<Output = Form>, Error>
where
    T: Serialize + MultipartPayload,
{
    let direct_upload_fields = collect_direct_upload_fields(val);
    let direct_file_fields = direct_upload_fields.iter().map(|(_, field)| *field).collect();

    let mut form = val.serialize(MultipartSerializer::new(direct_file_fields))?;
    let mut vec = Vec::with_capacity(1);
    val.copy_files(&mut |f| vec.push(f));

    let iter = vec.into_iter();

    let fut = async move {
        for file in iter {
            if file.needs_attach() {
                let id = file.id().to_owned();
                let field = direct_upload_fields
                    .iter()
                    .find_map(|(direct_id, field)| (direct_id == &id).then_some(*field))
                    .unwrap_or(&id);
                if let Some(part) = file.into_part(field.to_owned()) {
                    form = form.part(field.to_owned(), part.await);
                }
            }
        }

        form
    };

    Ok(fut)
}

fn collect_direct_upload_fields<T: ?Sized>(val: &T) -> Vec<(String, &'static str)>
where
    T: MultipartPayload,
{
    let mut direct_upload_fields = Vec::with_capacity(1);
    val.direct_upload_fields(&mut |field, file_id| {
        direct_upload_fields.push((file_id.to_owned(), field));
    });
    direct_upload_fields
}

#[cfg(test)]
mod tests {
    use tokio::fs::File;

    use super::{collect_direct_upload_fields, to_form_ref};
    use crate::{
        payloads::{self, setters::*},
        types::{
            ChatId, InputFile, InputMedia, InputMediaAnimation, InputMediaAudio,
            InputMediaDocument, InputMediaPhoto, InputMediaVideo, InputSticker, MessageEntity,
            MessageEntityKind, ParseMode, StickerFormat, UserId,
        },
    };

    // https://github.com/teloxide/teloxide/issues/473
    #[tokio::test]
    async fn issue_473() {
        to_form_ref(
            &payloads::SendPhoto::new(ChatId(0), InputFile::file_id("0".into())).caption_entities(
                [MessageEntity { kind: MessageEntityKind::Url, offset: 0, length: 0 }],
            ),
        )
        .unwrap()
        .await;
    }

    #[test]
    fn direct_upload_fields_only_include_top_level_input_files() {
        let photo = payloads::SendPhoto::new(ChatId(0), InputFile::memory(&b"photo"[..]));
        let photo_fields = collect_direct_upload_fields(&photo);
        assert_eq!(photo_fields.len(), 1);
        assert_eq!(photo_fields[0].1, "photo");

        let photo_by_id = payloads::SendPhoto::new(ChatId(0), InputFile::file_id("id".into()));
        assert!(collect_direct_upload_fields(&photo_by_id).is_empty());

        let sticker = payloads::AddStickerToSet::new(
            UserId(0),
            "name",
            InputSticker {
                sticker: InputFile::memory(&b"sticker"[..]),
                emoji_list: vec!["✈️⚙️".to_owned()],
                keywords: vec![],
                mask_position: None,
                format: StickerFormat::Static,
            },
        );
        assert!(collect_direct_upload_fields(&sticker).is_empty());

        let media_group = payloads::SendMediaGroup::new(
            ChatId(0),
            [InputMedia::Photo(InputMediaPhoto::new(InputFile::memory(&b"group-photo"[..])))],
        );
        assert!(collect_direct_upload_fields(&media_group).is_empty());
    }

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
        .await;
    }

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
        .await;
    }

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
        .await;
    }

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
