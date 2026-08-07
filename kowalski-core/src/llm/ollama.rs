use super::provider::{ChatOutcome, LLMProvider, TokenStream, ToolDefinition};
use crate::agent::types::ChatRequest;
use crate::conversation::{FunctionCall, Message, ToolCall};
use crate::error::KowalskiError;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;

pub struct OllamaProvider {
    base_url: String,
    client: Client,
    native_tools: bool,
}

impl OllamaProvider {
    pub fn new(host: &str, port: u16) -> Self {
        let base_url = format!("http://{}:{}", host, port);
        let client = Client::new();
        Self {
            base_url,
            client,
            native_tools: false,
        }
    }

    /// Opt in to native tool calling (`[llm] native_tools`); requires a tool-capable model.
    pub fn with_native_tools(mut self, enabled: bool) -> Self {
        self.native_tools = enabled;
        self
    }

    /// Single owner of Ollama `ChatRequest` construction — every request path (chat,
    /// stream, native tools) goes through here.
    fn build_request(
        &self,
        model: &str,
        messages: &[Message],
        stream: bool,
        tools: Option<&[ToolDefinition]>,
    ) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream,
            temperature: 0.7,
            max_tokens: 2048,
            tools: tools
                .map(|defs| serde_json::Value::Array(defs.iter().map(|d| d.wire_json()).collect())),
        }
    }

    fn troubleshoot_connect(&self, endpoint: &str, err: &reqwest::Error) -> String {
        format!(
            "Cannot reach Ollama at {} (requested {}): {}.\n\
             What to check:\n\
             - Is the Ollama daemon running? (`ollama serve`, or start the Ollama app.)\n\
             - Does Kowalski `config.toml` `[ollama]` host/port match where Ollama listens? (default http://127.0.0.1:11434)\n\
             - From the same machine: `curl -s {}/api/tags` should return JSON, not \"connection refused\".\n\
             - If you use a remote Ollama, confirm firewall/VPN and that `OLLAMA_HOST` on the Ollama side allows your client.",
            self.base_url, endpoint, err, self.base_url
        )
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(&self, model: &str, messages: &[Message]) -> Result<String, KowalskiError> {
        let url = format!("{}/api/chat", self.base_url);
        let request = self.build_request(model, messages, false, None);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| KowalskiError::Server(self.troubleshoot_connect(&url, &e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Server(format!(
                "Ollama returned HTTP {} from {} for model `{}`. Body: {}.\n\
                 What to check:\n\
                 - Model pulled? `ollama pull {}`\n\
                 - Ollama logs for stack traces (terminal where `ollama serve` runs).",
                status,
                url,
                model,
                error_text.trim(),
                model
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| {
                KowalskiError::Server(format!(
                    "Ollama returned success HTTP but invalid JSON from {}: {}. Raw response may be truncated in logs.",
                    url, e
                ))
            })?;

        let content = response_json["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                KowalskiError::Server(format!(
                    "No `message.content` in Ollama JSON from {}. Keys present: {:?}. Full body (trimmed): {:.500}",
                    url,
                    response_json
                        .as_object()
                        .map(|o| o.keys().cloned().collect::<Vec<_>>())
                        .unwrap_or_default(),
                    response_json.to_string()
                ))
            })?
            .to_string();

        Ok(content)
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        let url = format!("{}/api/embeddings", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "model": "nomic-embed-text",
                "prompt": text
            }))
            .send()
            .await
            .map_err(|e| KowalskiError::Memory(self.troubleshoot_connect(&url, &e)))?;

        let status = response.status();
        if !status.is_success() {
            let t = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Memory(format!(
                "Ollama embedding HTTP {} from {}: {}",
                status.as_u16(),
                url,
                t.trim()
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| KowalskiError::Memory(format!("Failed to parse embedding JSON: {}", e)))?;

        let embedding = json["embedding"]
            .as_array()
            .ok_or(KowalskiError::Memory(
                "No embedding field in response".to_string(),
            ))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_native_tools(&self, _model: &str) -> bool {
        self.native_tools
    }

    async fn chat_with_tool_defs(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<ChatOutcome, KowalskiError> {
        let url = format!("{}/api/chat", self.base_url);
        let request = self.build_request(model, messages, false, Some(tools));

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| KowalskiError::Server(self.troubleshoot_connect(&url, &e)))?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(KowalskiError::Server(format!(
                "Ollama returned HTTP {} from {} for model `{}` (tool-calling request). Body: {}.\n\
                 What to check:\n\
                 - Model pulled and tool-capable? `ollama pull {}` (older models reject `tools`).\n\
                 - Ollama logs for stack traces (terminal where `ollama serve` runs).",
                status,
                url,
                model,
                error_text.trim(),
                model
            )));
        }

        let response_json: serde_json::Value = response.json().await.map_err(|e| {
            KowalskiError::Server(format!(
                "Ollama returned success HTTP but invalid JSON from {}: {}. Raw response may be truncated in logs.",
                url, e
            ))
        })?;

        parse_chat_outcome(&response_json, &url)
    }

    fn chat_stream(&self, model: &str, messages: Vec<Message>) -> TokenStream<'_> {
        let url = format!("{}/api/chat", self.base_url);
        let base_url = self.base_url.clone();
        let request = self.build_request(model, &messages, true, None);
        let client = self.client.clone();
        Box::pin(async_stream::stream! {
            let response = match client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield Err(KowalskiError::Server(format!(
                        "Cannot reach Ollama at {} (stream {}): {}.\n\
                         What to check: same as non-stream — run `ollama serve`, match `[ollama]` in config.toml, `curl -s {}/api/tags`.",
                        base_url, url, e, base_url
                    )));
                    return;
                }
            };
            let status = response.status();
            if !status.is_success() {
                let t = response.text().await.unwrap_or_default();
                yield Err(KowalskiError::Server(format!(
                    "Ollama stream returned HTTP {} from {}: {}",
                    status.as_u16(),
                    url,
                    t.trim()
                )));
                return;
            }
            let mut buf: Vec<u8> = Vec::new();
            let mut bytes_stream = response.bytes_stream();
            while let Some(chunk) = bytes_stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(KowalskiError::Server(format!(
                            "Ollama stream read error from {}: {}",
                            url, e
                        )));
                        return;
                    }
                };
                buf.extend_from_slice(&chunk);
                while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let raw: Vec<u8> = buf.drain(..=pos).collect();
                    let line = String::from_utf8_lossy(&raw);
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    let v: serde_json::Value = match serde_json::from_str(line) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if let Some(c) = v["message"]["content"].as_str()
                        && !c.is_empty() {
                            yield Ok(c.to_string());
                        }
                }
            }
        })
    }
}

/// Map an Ollama `/api/chat` response body to a [`ChatOutcome`].
///
/// Ollama emits `message.tool_calls` as `[{"function": {"name", "arguments": {…}}}]` with no
/// call ids; ids are synthesized (`call_<index>`) so callers can pair tool results uniformly
/// across providers.
fn parse_chat_outcome(
    response_json: &serde_json::Value,
    url: &str,
) -> Result<ChatOutcome, KowalskiError> {
    let message = &response_json["message"];
    let content = message["content"].as_str().unwrap_or_default();

    if let Some(raw_calls) = message["tool_calls"].as_array()
        && !raw_calls.is_empty()
    {
        let mut calls = Vec::with_capacity(raw_calls.len());
        for (index, raw) in raw_calls.iter().enumerate() {
            let function = &raw["function"];
            let name = function["name"].as_str().ok_or_else(|| {
                KowalskiError::Server(format!(
                    "Ollama tool call #{} from {} has no `function.name`. Entry: {:.300}",
                    index, url, raw
                ))
            })?;
            let arguments = match &function["arguments"] {
                serde_json::Value::String(s) => {
                    serde_json::from_str(s).unwrap_or_else(|_| serde_json::Value::String(s.clone()))
                }
                serde_json::Value::Null => serde_json::json!({}),
                other => other.clone(),
            };
            let id = raw["id"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| format!("call_{}", index));
            calls.push(ToolCall {
                id,
                function: FunctionCall {
                    name: name.to_string(),
                    arguments,
                },
            });
        }
        return Ok(ChatOutcome::ToolCalls {
            content: if content.is_empty() {
                None
            } else {
                Some(content.to_string())
            },
            calls,
        });
    }

    if message["content"].is_string() {
        return Ok(ChatOutcome::Text(content.to_string()));
    }

    Err(KowalskiError::Server(format!(
        "No `message.content` or `message.tool_calls` in Ollama JSON from {}. Keys present: {:?}. Full body (trimmed): {:.500}",
        url,
        response_json
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        response_json.to_string()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider() -> OllamaProvider {
        OllamaProvider::new("localhost", 11434).with_native_tools(true)
    }

    fn weather_tool() -> ToolDefinition {
        ToolDefinition {
            name: "get_current_weather".to_string(),
            description: "Get the current weather for a location".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string", "description": "The city name" }
                },
                "required": ["location"]
            }),
        }
    }

    #[test]
    fn request_with_tools_serializes_function_format() {
        let messages = vec![Message::text("user", "What is the weather in Paris?")];
        let request =
            provider().build_request("llama3.2", &messages, false, Some(&[weather_tool()]));
        let wire = serde_json::to_value(&request).unwrap();

        assert_eq!(wire["model"], "llama3.2");
        assert_eq!(wire["stream"], false);
        let tools = wire["tools"].as_array().expect("tools array on the wire");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "get_current_weather");
        assert_eq!(
            tools[0]["function"]["parameters"]["required"],
            json!(["location"])
        );
    }

    #[test]
    fn request_without_tools_omits_tools() {
        let messages = vec![Message::text("user", "hi")];
        let request = provider().build_request("llama3.2", &messages, false, None);
        let wire = serde_json::to_value(&request).unwrap();
        assert!(wire["tools"].is_null());
    }

    #[test]
    fn tool_role_message_wire_format() {
        let messages = vec![
            Message::text("user", "What is the weather in Paris?"),
            Message::assistant_tool_calls(
                "",
                vec![ToolCall {
                    id: "call_0".to_string(),
                    function: FunctionCall {
                        name: "get_current_weather".to_string(),
                        arguments: json!({"location": "Paris"}),
                    },
                }],
            ),
            Message::tool_result("call_0", "12 degrees and cloudy"),
        ];
        let request =
            provider().build_request("llama3.2", &messages, false, Some(&[weather_tool()]));
        let wire = serde_json::to_value(&request).unwrap();

        let assistant = &wire["messages"][1];
        assert_eq!(assistant["role"], "assistant");
        assert_eq!(
            assistant["tool_calls"][0]["function"]["name"],
            "get_current_weather"
        );
        let tool_msg = &wire["messages"][2];
        assert_eq!(tool_msg["role"], "tool");
        assert_eq!(tool_msg["content"], "12 degrees and cloudy");
        assert_eq!(tool_msg["tool_call_id"], "call_0");
        // Plain messages stay clean: no null tool fields on the wire.
        let user_msg = wire["messages"][0].as_object().unwrap();
        assert!(!user_msg.contains_key("tool_calls"));
        assert!(!user_msg.contains_key("tool_call_id"));
    }

    // Captured Ollama /api/chat wire fixture: tool-capable model answering with a call.
    #[test]
    fn response_with_tool_calls_parses_structured() {
        let fixture = json!({
            "model": "llama3.2",
            "created_at": "2026-08-07T10:00:00.000000Z",
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    { "function": { "name": "get_current_weather", "arguments": { "location": "Paris" } } }
                ]
            },
            "done_reason": "stop",
            "done": true
        });
        match parse_chat_outcome(&fixture, "http://test/api/chat").unwrap() {
            ChatOutcome::ToolCalls { content, calls } => {
                assert_eq!(content, None);
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].id, "call_0");
                assert_eq!(calls[0].function.name, "get_current_weather");
                assert_eq!(calls[0].function.arguments, json!({"location": "Paris"}));
            }
            other => panic!("expected ToolCalls, got {:?}", other),
        }
    }

    #[test]
    fn response_with_string_arguments_parses_to_json() {
        let fixture = json!({
            "message": {
                "role": "assistant",
                "content": "Checking now.",
                "tool_calls": [
                    { "function": { "name": "get_current_weather", "arguments": "{\"location\": \"Paris\"}" } }
                ]
            },
            "done": true
        });
        match parse_chat_outcome(&fixture, "http://test/api/chat").unwrap() {
            ChatOutcome::ToolCalls { content, calls } => {
                assert_eq!(content.as_deref(), Some("Checking now."));
                assert_eq!(calls[0].function.arguments, json!({"location": "Paris"}));
            }
            other => panic!("expected ToolCalls, got {:?}", other),
        }
    }

    #[test]
    fn response_without_tool_calls_parses_text() {
        let fixture = json!({
            "message": { "role": "assistant", "content": "It is sunny." },
            "done": true
        });
        match parse_chat_outcome(&fixture, "http://test/api/chat").unwrap() {
            ChatOutcome::Text(text) => assert_eq!(text, "It is sunny."),
            other => panic!("expected Text, got {:?}", other),
        }
    }

    #[test]
    fn response_missing_message_is_an_error() {
        let fixture = json!({ "done": true });
        assert!(parse_chat_outcome(&fixture, "http://test/api/chat").is_err());
    }

    #[test]
    fn native_tools_flag_gates_support() {
        assert!(!OllamaProvider::new("localhost", 11434).supports_native_tools("llama3.2"));
        assert!(provider().supports_native_tools("llama3.2"));
    }
}
