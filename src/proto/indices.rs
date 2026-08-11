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
    /// HAR-observed value `6` — new dialog flag, always `[1]` in both fresh and
    /// continuation requests.
    pub const SLOT_NEW_DIALOG_FLAG: usize = 6;
    /// HAR-observed value `7` — request mode / category, sent as `1`.
    pub const SLOT_REQUEST_MODE: usize = 7;
    /// HAR-observed value `10` — protocol version, sent as `1`.
    pub const SLOT_PROTOCOL_VERSION: usize = 10;
    /// HAR-observed value `11` — protocol subversion, sent as `0`.
    pub const SLOT_PROTOCOL_SUBVERSION: usize = 11;
    /// HAR-observed value `17` — turn counter; fresh `[[0]]`, continuation
    /// `[[1]]`.
    pub const SLOT_TURN_COUNTER: usize = 17;
    /// HAR-observed value `18` — turn counter mode, sent as `0`.
    pub const SLOT_TURN_COUNTER_MODE: usize = 18;
    /// HAR-observed value `27` — streaming flag; the frontend always sends `1`.
    pub const SLOT_STREAMING_FLAG: usize = 27;
    /// Slot that carries the request category enum wrapped in an array.
    pub const SLOT_REQUEST_CATEGORY: usize = 30;
    /// HAR-observed value `41` — mode picker / thinking flag, sent as `[1]`.
    pub const SLOT_MODE_PICKER: usize = 41;
    /// HAR-observed value `53` — tool execution mode, sent as `0`.
    pub const SLOT_TOOL_EXECUTION_MODE: usize = 53;
    /// HAR-observed value `59` — request UUID, matches `_reqid` query and the
    /// `525005358` header.
    pub const SLOT_REQUEST_UUID: usize = 59;
    /// HAR-observed value `61` — empty context list, sent as `[]`.
    pub const SLOT_EMPTY_CONTEXT_LIST: usize = 61;
    /// HAR-observed value `66` — unused placeholder, sent as `null`.
    pub const SLOT_UNUSED_PLACEHOLDER: usize = 66;
    /// HAR-observed value `68` — response version, sent as `2`.
    pub const SLOT_RESPONSE_VERSION: usize = 68;
    /// HAR-observed value `79` — candidate count, sent as `3`.
    pub const SLOT_CANDIDATE_COUNT: usize = 79;
    /// Slot for the thinking level enum value.
    pub const SLOT_THINKING_LEVEL: usize = 80;
    /// Slot used for tool declarations when function calling is enabled.
    pub const SLOT_TOOL_DECLARATIONS: usize = 89;
    /// HAR-observed value `91` — safety filter level, sent as `0`.
    pub const SLOT_SAFETY_FILTER_LEVEL: usize = 91;
    /// HAR-observed value `96` — fresh conversation flag; `1` for fresh, `0`
    /// for continuation.
    pub const SLOT_FRESH_CONVERSATION_FLAG: usize = 96;
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
    ///
    /// This is a WIZ map key, not a semantic identifier, so it remains a
    /// numeric string literal.
    pub const META_TOKEN_KEY_26: &str = "26";
    /// Alternate JSON object key for the continuation token.
    ///
    /// This is a WIZ map key, not a semantic identifier, so it remains a
    /// numeric string literal.
    pub const META_TOKEN_KEY_21: &str = "21";

    /// RPC id string marker for batchexecute / StreamGenerate entries.
    pub use crate::constants::transport::RPC_FRAME_MARKER as RPC_ID;

    /// Index of the inner payload string in a batchexecute entry.
    pub const PAYLOAD: usize = 2;
    /// Alternate index of the inner payload string in a batchexecute entry.
    pub const PAYLOAD_ALT: usize = 3;

    /// Minimum expected length of the main `StreamGenerate` entry array.
    pub const MAIN_ENTRY_MIN_LEN: usize = 5;

    /// Expected length of the WAA context uuid array inside slot 15 data.
    pub const WAA_CONTEXT_LEN: usize = 15;
}
