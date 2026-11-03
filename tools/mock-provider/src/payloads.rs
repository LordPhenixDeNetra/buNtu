//! Réponses préconstruites une fois au démarrage.
//!
//! Chaque réponse est un `Bytes` statique : une requête ne coûte qu'un clone de compteur
//! de références, jamais une sérialisation. C'est la condition pour que le simulateur ne
//! soit pas le goulot des tests ni des benchmarks.

use bytes::Bytes;

/// Modèle renvoyé dans toutes les réponses (le simulateur n'analyse pas le modèle demandé).
pub const MODEL: &str = "mock-model";

const CHAT_JSON: &str = concat!(
    r#"{"id":"chatcmpl-mock","object":"chat.completion","created":0,"model":"mock-model","#,
    r#""choices":[{"index":0,"message":{"role":"assistant","content":"token token "},"#,
    r#""finish_reason":"stop"}],"usage":{"prompt_tokens":2,"completion_tokens":2,"total_tokens":4}}"#
);

const CHAT_CHUNK: &str = concat!(
    "data: ",
    r#"{"id":"chatcmpl-mock","object":"chat.completion.chunk","created":0,"model":"mock-model","#,
    r#""choices":[{"index":0,"delta":{"content":"token "},"finish_reason":null}]}"#,
    "\n\n"
);

const CHAT_FINAL: &str = concat!(
    "data: ",
    r#"{"id":"chatcmpl-mock","object":"chat.completion.chunk","created":0,"model":"mock-model","#,
    r#""choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
    "\n\n"
);

const CHAT_DONE: &str = "data: [DONE]\n\n";

const EMBEDDINGS_JSON: &str = concat!(
    r#"{"object":"list","data":[{"object":"embedding","index":0,"embedding":["#,
    "0.0,0.1,0.2,0.3,0.4,0.5,0.6,0.7",
    r#"]}],"model":"mock-model","usage":{"prompt_tokens":1,"total_tokens":1}}"#
);

const ANTHROPIC_JSON: &str = concat!(
    r#"{"id":"msg_mock","type":"message","role":"assistant","model":"mock-model","#,
    r#""content":[{"type":"text","text":"token token "}],"stop_reason":"end_turn","#,
    r#""stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":2}}"#
);

const ANTHROPIC_START: &str = concat!(
    "event: message_start\ndata: ",
    r#"{"type":"message_start","message":{"id":"msg_mock","type":"message","role":"assistant","#,
    r#""content":[],"model":"mock-model","stop_reason":null,"stop_sequence":null,"#,
    r#""usage":{"input_tokens":10,"output_tokens":1}}}"#,
    "\n\n"
);

const ANTHROPIC_BLOCK_START: &str = concat!(
    "event: content_block_start\ndata: ",
    r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
    "\n\n"
);

const ANTHROPIC_DELTA: &str = concat!(
    "event: content_block_delta\ndata: ",
    r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"token "}}"#,
    "\n\n"
);

const ANTHROPIC_BLOCK_STOP: &str = concat!(
    "event: content_block_stop\ndata: ",
    r#"{"type":"content_block_stop","index":0}"#,
    "\n\n"
);

const ANTHROPIC_MESSAGE_DELTA: &str = concat!(
    "event: message_delta\ndata: ",
    r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":2}}"#,
    "\n\n"
);

const ANTHROPIC_STOP: &str = concat!(
    "event: message_stop\ndata: ",
    r#"{"type":"message_stop"}"#,
    "\n\n"
);

const ERROR_JSON: &str = concat!(
    r#"{"error":{"message":"simulated provider error","type":"mock_error","#,
    r#""code":"mock_status"}}"#
);

/// Toutes les réponses du simulateur, construites une seule fois.
pub struct Payloads {
    pub chat_json: Bytes,
    pub chat_chunk: Bytes,
    pub chat_final: Bytes,
    pub chat_done: Bytes,
    pub embeddings_json: Bytes,
    pub anthropic_json: Bytes,
    pub anthropic_start: Bytes,
    pub anthropic_block_start: Bytes,
    pub anthropic_delta: Bytes,
    pub anthropic_block_stop: Bytes,
    pub anthropic_message_delta: Bytes,
    pub anthropic_stop: Bytes,
    pub error_json: Bytes,
}

impl Payloads {
    pub fn build() -> Self {
        Self {
            chat_json: Bytes::from_static(CHAT_JSON.as_bytes()),
            chat_chunk: Bytes::from_static(CHAT_CHUNK.as_bytes()),
            chat_final: Bytes::from_static(CHAT_FINAL.as_bytes()),
            chat_done: Bytes::from_static(CHAT_DONE.as_bytes()),
            embeddings_json: Bytes::from_static(EMBEDDINGS_JSON.as_bytes()),
            anthropic_json: Bytes::from_static(ANTHROPIC_JSON.as_bytes()),
            anthropic_start: Bytes::from_static(ANTHROPIC_START.as_bytes()),
            anthropic_block_start: Bytes::from_static(ANTHROPIC_BLOCK_START.as_bytes()),
            anthropic_delta: Bytes::from_static(ANTHROPIC_DELTA.as_bytes()),
            anthropic_block_stop: Bytes::from_static(ANTHROPIC_BLOCK_STOP.as_bytes()),
            anthropic_message_delta: Bytes::from_static(ANTHROPIC_MESSAGE_DELTA.as_bytes()),
            anthropic_stop: Bytes::from_static(ANTHROPIC_STOP.as_bytes()),
            error_json: Bytes::from_static(ERROR_JSON.as_bytes()),
        }
    }
}
