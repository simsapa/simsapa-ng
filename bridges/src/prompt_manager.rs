use std::thread;
use core::pin::Pin;

use cxx_qt_lib::QString;
use cxx_qt::Threading;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

// use simsapa_backend::logger::info;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[namespace = "prompt_manager"]
        type PromptManager = super::PromptManagerRust;
    }

    impl cxx_qt::Threading for PromptManager{}

    extern "RustQt" {
        #[qinvokable]
        fn prompt_request(self: Pin<&mut PromptManager>, paragraph_idx: usize, translation_idx: usize, model: &QString, prompt: &QString);

        #[qsignal]
        #[cxx_name = "promptResponse"]
        fn prompt_response(self: Pin<&mut PromptManager>, paragraph_idx: usize, translation_idx: usize, model: QString, response: QString);
    }
}

#[derive(Default, Copy, Clone)]
pub struct PromptManagerRust;

impl qobject::PromptManager {
    fn prompt_request(self: Pin<&mut Self>, paragraph_idx: usize, translation_idx: usize, model: &QString, prompt: &QString) {
        let qt_thread = self.qt_thread();

        // FIXME: read OPENROUTER_API_KEY from db
        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| String::from(""));

        let api_url = "https://openrouter.ai/api/v1/chat/completions".to_string();

        let prompt_text = prompt.to_string();
        let model_text = model.to_string();

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            let client = Client::new();

            let request_body = ChatRequest {
                model: model_text.clone(),
                messages: vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: prompt_text,
                    }
                ],
                max_tokens: None,
                temperature: None,
            };

            let response_content = match make_api_request(
                &client,
                &api_url,
                &api_key,
                request_body
            ) {
                Ok(content) => content,
                Err(e) => format!("Error: {}", e),
            };

            // Emit signal with the prompt response
            qt_thread.queue(move |mut qo| {
                qo.as_mut().prompt_response(
                    paragraph_idx,
                    translation_idx,
                    QString::from(model_text),
                    QString::from(response_content));
            }).unwrap();
        }); // end of thread
    }
}

fn make_api_request(
    client: &Client,
    api_url: &str,
    api_key: &str,
    request_body: ChatRequest,
) -> Result<String, String> {
    let json_body = match serde_json::to_string(&request_body) {
        Ok(json) => json,
        Err(e) => return Err(format!("Failed to serialize request: {}", e)),
    };

    let auth_header = format!("Bearer {}", api_key);

    let response = client
        .post(api_url)
        .header(CONTENT_TYPE, "application/json")
        .header(AUTHORIZATION, auth_header)
        .body(json_body)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();

    let response_text = response
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    if !status.is_success() {
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
            return Err(format!("API Error: {}", error_response.error.message));
        } else {
            return Err(format!("HTTP Error {}: {}", status, response_text));
        }
    }

    let chat_response: ChatResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse response: {}. Raw response: {}", e, response_text))?;

    // Check for API-level errors in the response
    if let Some(error) = chat_response.error {
        return Err(format!("API Error: {}", error.message));
    }

    chat_response
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .ok_or_else(|| "No response content received".to_string())
}

// Structures for OpenRouter API
#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    error: Option<ErrorInfo>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize, Debug)]
struct ErrorInfo {
    message: String,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: ErrorInfo,
}
