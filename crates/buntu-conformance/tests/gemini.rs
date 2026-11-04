use buntu_conformance::conformance_suite;
use buntu_providers::gemini::GeminiProvider;

conformance_suite!(
    GeminiProvider::default(),
    "gemini",
    text_simple,
    system_multi_turn,
    stream,
    tool_call,
    parallel_tools,
    tool_result,
    image_input,
    json_output,
    length_stop,
    content_filter,
    error_400,
    error_401,
    error_429,
    error_500,
);
