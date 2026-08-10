//! Named indices for the Gemini web frontend WIZ protocol.
//!
//! The `StreamGenerate` request is a 97-slot JSON array and the responses are
//! nested arrays whose positions are otherwise magic numbers. This module
//! centralizes the indices used by [`crate::proto::slots`] (request building) and
//! [`crate::proto::parser`] (response parsing) so the builder and parser cannot
//! drift.

/// Indices used when constructing the 97-slot `StreamGenerate` request list.
pub mod builder {
    /// Slot for the prompt / attachments tuple.
    pub const SLOT_PROMPT: usize = 0;
    /// Slot for the `[language]` array.
    pub const SLOT_LANGUAGE: usize = 1;
    /// Slot for the multi-turn conversation state tuple.
    pub const SLOT_CONVERSATION_STATE: usize = 2;
    /// Slot for the WAA / attestation token.
    pub const SLOT_WAA_TOKEN: usize = 3;
    /// Slot for the request nonce.
    pub const SLOT_NONCE: usize = 4;
    /// Slot used when continuing a conversation.
    pub const SLOT_CONTINUATION_FLAG: usize = 6;
    /// Slot that controls the request category / mode.
    pub const SLOT_CATEGORY: usize = 7;
    /// Slot for the request UUID.
    pub const SLOT_REQUEST_UUID: usize = 10;
    /// Slot that toggles between fresh and continuing conversation state.
    pub const SLOT_FRESH_FLAG: usize = 11;
    /// Slot that carries the request category enum wrapped in an array.
    pub const SLOT_REQUEST_CATEGORY: usize = 30;
    /// Slot that enables the thinking / reasoning block.
    pub const SLOT_THINKING_FLAG: usize = 41;
    /// Slot for the thinking level enum value.
    pub const SLOT_THINKING_LEVEL: usize = 80;
    /// Slot that distinguishes new conversations from continuations.
    pub const SLOT_CONVERSATION_TYPE: usize = 96;
    /// Slot used for tool declarations when function calling is enabled.
    pub const SLOT_TOOL_DECLARATIONS: usize = 89;
}

/// Indices used when parsing WIZ / batchexecute responses.
pub mod parser {
    /// Index of the text chunk list within a candidate part.
    pub const PART_TEXT: usize = 1;
    /// Index of the reasoning block within a candidate part.
    pub const PART_THINKING: usize = 37;
    /// Index of the function-call block within a candidate part.
    pub const PART_FUNCTION_CALL: usize = 7;
    /// Index of the candidate / response part id.
    pub const PART_ID: usize = 0;

    /// Index of the candidate parts array within the main response entry.
    pub const CANDIDATE_PARTS: usize = 4;
    /// Index of the `[conversation_id, response_id]` array in the main entry.
    pub const CONVERSATION_IDS: usize = 1;

    /// JSON object key that holds the continuation token in meta responses.
    pub const META_TOKEN_KEY_26: &str = "26";
    /// Alternate JSON object key for the continuation token.
    pub const META_TOKEN_KEY_21: &str = "21";

    /// RPC id string marker for batchexecute / StreamGenerate entries.
    pub const RPC_ID: &str = "wrb.fr";

    /// Index of the inner payload string in a batchexecute entry.
    pub const PAYLOAD: usize = 2;
    /// Alternate index of the inner payload string in a batchexecute entry.
    pub const PAYLOAD_ALT: usize = 3;

    /// Minimum expected length of the main `StreamGenerate` entry array.
    pub const MAIN_ENTRY_MIN_LEN: usize = 5;

    /// Expected length of the WAA context uuid array inside slot 15 data.
    pub const WAA_CONTEXT_LEN: usize = 15;
}
