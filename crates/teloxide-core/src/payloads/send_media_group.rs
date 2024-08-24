//! Generated by `codegen_payloads`, do not edit by hand.

use serde::Serialize;

use crate::types::{
    BusinessConnectionId, InputMedia, Message, Recipient, ReplyParameters, ThreadId,
};

impl_payload! {
    /// Use this method to send a group of photos, videos, documents or audios as an album. Documents and audio files can be only grouped in an album with messages of the same type. On success, an array of [`Message`]s that were sent is returned.
    ///
    /// ## Captions
    ///
    /// You may want to set a "global" caption which renders in the message, underneath the set of media. However, global captions for a media group are not part of the Telegram API. They result from how the official clients (at least) render a media group where only one [`InputMedia`] has a caption set. That captioned [`InputMedia`] may be in any position of the group.
    ///
    /// In order to set a "global" caption of a media group, set a caption for a single [`InputMedia`] in the group with the contents you wish to display underneath all media.
    ///
    /// If multiple [`InputMedia`] have captions, including identical ones, the official clients will not render a global caption underneath the group. Each individual media will keep its own caption however, which can be shown by the client when viewing the media individually, or by separating the media in its own message (for example by forwarding a single media from the media group).
    ///
    /// [`Message`]: crate::types::Message
    /// [`InputMedia`]: crate::types::InputMedia
    #[derive(Debug, Clone, Serialize)]
    pub SendMediaGroup (SendMediaGroupSetters) => Vec<Message> {
        required {
            /// Unique identifier for the target chat or username of the target channel (in the format `@channelusername`)
            pub chat_id: Recipient [into],
            /// A JSON-serialized array describing messages to be sent, must include 2-10 items
            pub media: Vec<InputMedia> [collect],
        }
        optional {
            /// Unique identifier of the business connection on behalf of which the message will be sent
            pub business_connection_id: BusinessConnectionId,
            /// Unique identifier for the target message thread (topic) of the forum; for forum supergroups only
            pub message_thread_id: ThreadId,
            /// Sends the message [silently]. Users will receive a notification with no sound.
            ///
            /// [silently]: https://telegram.org/blog/channels-2-0#silent-messages
            pub disable_notification: bool,
            /// Protects the contents of sent messages from forwarding and saving
            pub protect_content: bool,
            /// Description of the message to reply to
            pub reply_parameters: ReplyParameters,
        }
    }
}
