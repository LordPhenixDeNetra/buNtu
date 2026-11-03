use buntu_conformance::{BrokenProvider, FakeProvider, conformance_suite};

conformance_suite!(
    FakeProvider::default(),
    "fake",
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

/// Preuve exigée par la fiche : une traduction volontairement fausse doit être détectée.
#[test]
fn broken_provider_is_detected() {
    let provider = BrokenProvider::new();
    for case in ["text_simple", "length_stop", "tool_call"] {
        let result = buntu_conformance::block_on_case(&provider, "fake", case);
        assert!(result.is_err(), "le cas « {case} » aurait dû échouer");
    }
}
