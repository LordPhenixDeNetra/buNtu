//! Tests de la résolution des cibles et du filtre de capacités.

use std::error::Error;

use buntu_core::config::CompiledConfig;
use buntu_core::config::{MapEnvironment, from_str};
use buntu_core::routing::{
    Capabilities, Target, filter_capabilities, required_capabilities, resolve,
};
use buntu_schema::{ContentPart, GatewayError, ImageDetail, ImageSource, Message, Request, Role};
use serde_json::Map;

const CONFIG: &str = r#"
[server]
listen = "127.0.0.1:8080"

[[providers]]
id = "a"
type = "openai"
keys = [{ secret = "env:KEY", weight = 1 }]

[[providers]]
id = "b"
type = "openai"
keys = [{ secret = "env:KEY", weight = 1 }]

[[routes]]
alias = "chat"
fallbacks = ["a/modele-a", "b/modele-b"]
"#;

fn config() -> Result<CompiledConfig, Box<dyn Error>> {
    Ok(from_str(
        CONFIG,
        &MapEnvironment::new([("KEY", "sk-test")]),
    )?)
}

fn labels(targets: &[Target]) -> Vec<String> {
    targets.iter().map(Target::label).collect()
}

fn message(parts: Vec<ContentPart>) -> Message {
    Message {
        role: Role::User,
        content: parts,
        name: None,
        extra: Map::new(),
    }
}

fn request(messages: Vec<Message>) -> Request {
    Request {
        model: "chat".to_owned(),
        messages,
        tools: Vec::new(),
        tool_choice: Default::default(),
        max_tokens: None,
        temperature: None,
        top_p: None,
        stop: Vec::new(),
        stream: false,
        response_format: None,
        extra: Map::new(),
    }
}

#[test]
fn alias_resolves_in_declared_order() -> Result<(), Box<dyn Error>> {
    let targets =
        resolve(&config()?, "chat", Capabilities::NONE).map_err(|error| error.to_string())?;
    assert_eq!(labels(&targets), vec!["a/modele-a", "b/modele-b"]);
    Ok(())
}

#[test]
fn direct_target_resolves() -> Result<(), Box<dyn Error>> {
    let targets =
        resolve(&config()?, "b/modele-c", Capabilities::NONE).map_err(|error| error.to_string())?;
    assert_eq!(labels(&targets), vec!["b/modele-c"]);
    Ok(())
}

#[test]
fn unknown_alias_is_rejected() -> Result<(), Box<dyn Error>> {
    match resolve(&config()?, "inconnu", Capabilities::NONE) {
        Err(GatewayError::InvalidRequest { message }) => {
            assert!(message.contains("inconnu"), "message : {message}");
        }
        other => panic!("erreur attendue, obtenu : {other:?}"),
    }
    Ok(())
}

#[test]
fn unknown_provider_is_rejected() -> Result<(), Box<dyn Error>> {
    match resolve(&config()?, "zzz/modele", Capabilities::NONE) {
        Err(GatewayError::InvalidRequest { message }) => {
            assert!(
                message.contains("fournisseur inconnu"),
                "message : {message}"
            );
        }
        other => panic!("erreur attendue, obtenu : {other:?}"),
    }
    Ok(())
}

#[test]
fn required_capabilities_detect_tools_and_vision() {
    // Un appel d'outil dans le contenu suffit à exiger la capacité outils.
    let with_tool_call = request(vec![message(vec![ContentPart::ToolCall(
        buntu_schema::ToolCall {
            id: "call-1".to_owned(),
            name: "meteo".to_owned(),
            arguments: serde_json::json!({}),
        },
    )])]);
    assert!(required_capabilities(&with_tool_call).tools);

    let with_image = request(vec![message(vec![ContentPart::Image {
        source: ImageSource::Url {
            url: "https://exemple.test/image.png".to_owned(),
        },
        detail: ImageDetail::Auto,
    }])]);
    assert!(required_capabilities(&with_image).vision);
    assert!(!required_capabilities(&with_image).tools);
}

#[test]
fn capability_filter_drops_known_unsupported_targets() {
    let mut capable = Target::new("a", "modele-a");
    capable.capabilities = Some(Capabilities {
        tools: true,
        vision: false,
    });
    let mut incapable = Target::new("b", "modele-b");
    incapable.capabilities = Some(Capabilities::NONE);
    let unknown = Target::new("c", "modele-c");

    let required = Capabilities {
        tools: true,
        vision: false,
    };
    let kept = filter_capabilities(vec![capable, incapable, unknown], required);
    assert_eq!(labels(&kept), vec!["a/modele-a", "c/modele-c"]);

    let mut incapable = Target::new("b", "modele-b");
    incapable.capabilities = Some(Capabilities::NONE);
    let none = filter_capabilities(
        vec![incapable],
        Capabilities {
            tools: true,
            vision: false,
        },
    );
    assert!(none.is_empty());
}
