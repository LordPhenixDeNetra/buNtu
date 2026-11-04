//! Batterie commune de cas et attentes canoniques (T1.8).

use buntu_schema::{
    ContentPart, GatewayError, ImageDetail, ImageSource, Message, Request, Response, Role,
    StopReason, StreamEvent, Tool, ToolCall, ToolResult, Usage,
};
use serde_json::{Map, Value, json};

/// Nature d'un cas de conformité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseKind {
    Response,
    Stream,
    Error,
}

/// Un cas de la batterie commune.
pub struct Case {
    pub name: &'static str,
    pub kind: CaseKind,
    pub request: fn() -> Request,
}

/// Résultat canonique observé pour un cas.
pub enum Outcome {
    Response(Response),
    Stream(Vec<StreamEvent>),
    Error(GatewayError),
}

/// Les 13 cas exigés par la fiche T1.8.
pub fn cases() -> &'static [Case] {
    &[
        Case {
            name: "text_simple",
            kind: CaseKind::Response,
            request: text_request,
        },
        Case {
            name: "system_multi_turn",
            kind: CaseKind::Response,
            request: system_request,
        },
        Case {
            name: "stream",
            kind: CaseKind::Stream,
            request: stream_request,
        },
        Case {
            name: "tool_call",
            kind: CaseKind::Response,
            request: tool_request,
        },
        Case {
            name: "parallel_tools",
            kind: CaseKind::Response,
            request: tool_request,
        },
        Case {
            name: "tool_result",
            kind: CaseKind::Response,
            request: tool_result_request,
        },
        Case {
            name: "image_input",
            kind: CaseKind::Response,
            request: image_request,
        },
        Case {
            name: "json_output",
            kind: CaseKind::Response,
            request: text_request,
        },
        Case {
            name: "length_stop",
            kind: CaseKind::Response,
            request: text_request,
        },
        Case {
            name: "content_filter",
            kind: CaseKind::Response,
            request: text_request,
        },
        Case {
            name: "error_400",
            kind: CaseKind::Error,
            request: text_request,
        },
        Case {
            name: "error_401",
            kind: CaseKind::Error,
            request: text_request,
        },
        Case {
            name: "error_429",
            kind: CaseKind::Error,
            request: text_request,
        },
        Case {
            name: "error_500",
            kind: CaseKind::Error,
            request: text_request,
        },
    ]
}

pub fn find(name: &str) -> Option<&'static Case> {
    cases().iter().find(|case| case.name == name)
}

fn base_request() -> Request {
    Request {
        model: "fake-model".to_owned(),
        messages: vec![Message {
            role: Role::User,
            content: vec![ContentPart::Text {
                text: "Bonjour".to_owned(),
            }],
            name: None,
            extra: Map::new(),
        }],
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

fn text_request() -> Request {
    base_request()
}

fn system_request() -> Request {
    let mut request = base_request();
    request.messages.insert(
        0,
        Message {
            role: Role::System,
            content: vec![ContentPart::Text {
                text: "Tu réponds en une phrase.".to_owned(),
            }],
            name: None,
            extra: Map::new(),
        },
    );
    request
}

fn stream_request() -> Request {
    let mut request = base_request();
    request.stream = true;
    request
}

fn tool_request() -> Request {
    let mut request = base_request();
    request.tools.push(Tool {
        name: "meteo".to_owned(),
        description: Some("Donne la météo d'une ville.".to_owned()),
        parameters: json!({"type": "object", "properties": {"ville": {"type": "string"}}}),
        extra: Map::new(),
    });
    request
}

fn tool_result_request() -> Request {
    let mut request = base_request();
    request.messages.push(Message {
        role: Role::Tool,
        content: vec![ContentPart::ToolResult(ToolResult {
            tool_call_id: "call-1".to_owned(),
            content: "22 °C".to_owned(),
            is_error: false,
        })],
        name: None,
        extra: Map::new(),
    });
    request
}

fn image_request() -> Request {
    let mut request = base_request();
    // Image encodée en base64 : c'est la forme acceptée par les trois fournisseurs testés
    // (OpenAI, Anthropic, Gemini). Les URL restent couvertes par les tests unitaires.
    request.messages[0].content.push(ContentPart::Image {
        source: ImageSource::Base64 {
            media_type: "image/png".to_owned(),
            data: "AAAA".to_owned(),
        },
        detail: ImageDetail::Auto,
    });
    request
}

fn text_of(response: &Response) -> Result<String, String> {
    let message = response
        .choices
        .first()
        .map(|choice| &choice.message)
        .ok_or_else(|| "aucun choix".to_owned())?;
    let mut text = String::new();
    for part in &message.content {
        if let ContentPart::Text { text: value } = part {
            text.push_str(value);
        }
    }
    Ok(text)
}

fn tool_calls_of(response: &Response) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    if let Some(choice) = response.choices.first() {
        for part in &choice.message.content {
            if let ContentPart::ToolCall(call) = part {
                calls.push(call.clone());
            }
        }
    }
    calls
}

fn assert_stop(response: &Response, expected: &StopReason) -> Result<(), String> {
    match &response.stop_reason {
        Some(reason) if reason == expected => Ok(()),
        other => Err(format!("raison d'arrêt {other:?} au lieu de {expected:?}")),
    }
}

fn assert_usage(response: &Response, input: u32, output: u32) -> Result<(), String> {
    let expected = Usage {
        input_tokens: input,
        output_tokens: output,
        cached_input_tokens: 0,
        reasoning_tokens: 0,
    };
    if response.usage == expected {
        Ok(())
    } else {
        Err(format!(
            "usage {:?} au lieu de {expected:?}",
            response.usage
        ))
    }
}

/// Vérifie qu'un résultat canonique respecte les attentes du cas.
pub fn expect(name: &str, outcome: &Outcome) -> Result<(), String> {
    match name {
        "text_simple" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            if text_of(response)? != "Bonjour" {
                return Err(format!("texte inattendu : {}", text_of(response)?));
            }
            assert_stop(response, &StopReason::EndTurn)?;
            assert_usage(response, 3, 5)
        }
        "system_multi_turn" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            if text_of(response)? != "Deux tours" {
                return Err(format!("texte inattendu : {}", text_of(response)?));
            }
            assert_stop(response, &StopReason::EndTurn)?;
            assert_usage(response, 7, 4)
        }
        "stream" => {
            let Outcome::Stream(events) = outcome else {
                return Err("flux attendu".to_owned());
            };
            // Les événements d'usage varient selon le fournisseur (message_start Anthropic,
            // usageMetadata Gemini…) : la séquence testée est celle du contenu.
            let events: Vec<StreamEvent> = events
                .iter()
                .filter(|event| !matches!(event, StreamEvent::Usage { .. }))
                .cloned()
                .collect();
            let expected = [
                StreamEvent::Start {
                    id: "fake-1".to_owned(),
                    model: "fake-model".to_owned(),
                },
                StreamEvent::ContentDelta {
                    index: 0,
                    part: ContentPart::Text {
                        text: "Bon".to_owned(),
                    },
                },
                StreamEvent::ContentDelta {
                    index: 0,
                    part: ContentPart::Text {
                        text: "jour".to_owned(),
                    },
                },
                StreamEvent::Stop {
                    reason: StopReason::EndTurn,
                },
                StreamEvent::Done,
            ];
            if events.len() != expected.len() {
                return Err(format!(
                    "{} événements au lieu de {}",
                    events.len(),
                    expected.len()
                ));
            }
            for (event, wanted) in events.iter().zip(expected.iter()) {
                if event != wanted {
                    return Err(format!("événement {event:?} au lieu de {wanted:?}"));
                }
            }
            Ok(())
        }
        "tool_call" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            let calls = tool_calls_of(response);
            if calls.len() != 1 {
                return Err(format!("{} appels d'outil au lieu de 1", calls.len()));
            }
            if calls[0].name != "meteo" || calls[0].id != "call-1" {
                return Err(format!("appel inattendu : {:?}", calls[0]));
            }
            assert_stop(response, &StopReason::ToolCalls)
        }
        "parallel_tools" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            let calls = tool_calls_of(response);
            if calls.len() != 2 {
                return Err(format!("{} appels d'outil au lieu de 2", calls.len()));
            }
            assert_stop(response, &StopReason::ToolCalls)
        }
        "tool_result" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            if text_of(response)? != "Il fait beau" {
                return Err(format!("texte inattendu : {}", text_of(response)?));
            }
            assert_stop(response, &StopReason::EndTurn)
        }
        "image_input" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            if text_of(response)? != "Une image" {
                return Err(format!("texte inattendu : {}", text_of(response)?));
            }
            Ok(())
        }
        "json_output" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            let text = text_of(response)?;
            let value: Value = serde_json::from_str(&text)
                .map_err(|error| format!("sortie JSON invalide ({error}) : {text}"))?;
            if value == json!({"ok": true}) {
                Ok(())
            } else {
                Err(format!("JSON inattendu : {value}"))
            }
        }
        "length_stop" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            assert_stop(response, &StopReason::MaxTokens)?;
            assert_usage(response, 2, 1)
        }
        "content_filter" => {
            let Outcome::Response(response) = outcome else {
                return Err("réponse attendue".to_owned());
            };
            // Selon le fournisseur, le refus de contenu s'exprime par `ContentFilter` ou
            // `Refusal` : la batterie accepte les deux traductions canoniques.
            match &response.stop_reason {
                Some(StopReason::ContentFilter | StopReason::Refusal) => {}
                other => {
                    return Err(format!(
                        "raison d'arrêt {other:?} au lieu de ContentFilter/Refusal"
                    ));
                }
            }
            assert_usage(response, 2, 0)
        }
        "error_400" => match outcome {
            Outcome::Error(GatewayError::InvalidRequest { .. }) => Ok(()),
            other => Err(format!("erreur 400 mal traduite : {other:?}")),
        },
        "error_401" => match outcome {
            Outcome::Error(GatewayError::Authentication) => Ok(()),
            other => Err(format!("erreur 401 mal traduite : {other:?}")),
        },
        "error_429" => match outcome {
            Outcome::Error(GatewayError::ProviderRateLimited {
                retry_after: Some(delay),
            }) if *delay == std::time::Duration::from_secs(2) => Ok(()),
            other => Err(format!("erreur 429 mal traduite : {other:?}")),
        },
        "error_500" => match outcome {
            Outcome::Error(GatewayError::ProviderUnavailable { .. }) => Ok(()),
            other => Err(format!("erreur 500 mal traduite : {other:?}")),
        },
        other => Err(format!("cas inconnu : {other}")),
    }
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Outcome::Response(response) => {
                formatter.debug_tuple("Response").field(response).finish()
            }
            Outcome::Stream(events) => formatter.debug_tuple("Stream").field(events).finish(),
            Outcome::Error(error) => formatter.debug_tuple("Error").field(error).finish(),
        }
    }
}
