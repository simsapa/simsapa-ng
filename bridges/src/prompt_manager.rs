use std::thread;
use core::pin::Pin;

use cxx_qt_lib::QString;
use cxx_qt::Threading;
use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

use simsapa_backend::logger::error;
use simsapa_backend::get_app_data;
use crate::{markdown_to_html, clean_prompt};

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
        fn prompt_request(self: Pin<&mut PromptManager>, paragraph_idx: usize, translation_idx: usize, model_name: &QString, prompt: &QString);

        #[qinvokable]
        fn prompt_request_with_messages(self: Pin<&mut PromptManager>, sender_message_idx: usize, model_name: &QString, messages_json: &QString);

        #[qsignal]
        #[cxx_name = "promptResponse"]
        fn prompt_response(self: Pin<&mut PromptManager>, paragraph_idx: usize, translation_idx: usize, model_name: QString, response: QString, response_html: QString);

        #[qsignal]
        #[cxx_name = "promptResponseForMessages"]
        fn prompt_response_for_messages(self: Pin<&mut PromptManager>, sender_message_idx: usize, model_name: QString, response: QString);
    }
}

#[derive(Default, Copy, Clone)]
pub struct PromptManagerRust;

impl qobject::PromptManager {
    fn prompt_request(self: Pin<&mut Self>, paragraph_idx: usize, translation_idx: usize, model_name: &QString, prompt: &QString) {
        let qt_thread = self.qt_thread();

        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
            let app_data = get_app_data();
            app_data.get_api_key("OPENROUTER_API_KEY")
        });

        let prompt_text = prompt.to_string();
        let model_name_text = model_name.to_string();

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            let request_body = ChatRequest {
                model: model_name_text.clone(),
                messages: vec![
                    ChatMessage {
                        role: "user".to_string(),
                        content: prompt_text,
                    }
                ],
                max_tokens: None,
                temperature: None,
            };

            let response_content = match make_openrouter_api_request(
                &api_key,
                request_body
            ) {
                Ok(content) => content,
                Err(e) => format!("Error: {}", e),
            };

            let response_content_html = markdown_to_html(&response_content);

            // Emit signal with the prompt response
            qt_thread.queue(move |mut qo| {
                qo.as_mut().prompt_response(
                    paragraph_idx,
                    translation_idx,
                    QString::from(model_name_text),
                    QString::from(response_content.trim()),
                    QString::from(response_content_html.trim()));
            }).unwrap();
        }); // end of thread
    }

    fn prompt_request_with_messages(self: Pin<&mut Self>, sender_message_idx: usize, model_name: &QString, messages_json: &QString) {
        let qt_thread = self.qt_thread();

        let api_key = std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
            let app_data = get_app_data();
            app_data.get_api_key("OPENROUTER_API_KEY")
        });

        let messages: Vec<ChatMessage> = match serde_json::from_str(&messages_json.to_string()) {
            Ok(r) => r,
            Err(e) => {
                error(&format!("{}", e));
                return;
            }
        };
        let model_name_text = model_name.to_string();

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            let request_body = ChatRequest {
                model: model_name_text.clone(),
                messages,
                max_tokens: None,
                temperature: None,
            };

            let response_content = match make_openrouter_api_request(
                &api_key,
                request_body
            ) {
                Ok(content) => content,
                Err(e) => format!("Error: {}", e),
            };

            // Emit signal with the prompt response (HTML conversion now done client-side)
            qt_thread.queue(move |mut qo| {
                qo.as_mut().prompt_response_for_messages(
                    sender_message_idx,
                    QString::from(model_name_text),  // Add model name to identify which model responded
                    QString::from(response_content.trim()),  // Raw response without HTML conversion
                );
            }).unwrap();
        }); // end of thread
    }
}

fn make_openrouter_api_request(api_key: &str, request_body: ChatRequest) -> Result<String, String> {
    let api_url = "https://openrouter.ai/api/v1/chat/completions".to_string();

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(180)) // 3 minutes timeout
        .build()
        .expect("Failed to build HTTP client");

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
        .map_err(|e| {
            let msg = format!("HTTP request failed. Error kind: {:?}. Error: {}", e, e);
            error(&msg);
            msg
        })?;

    let status = response.status();

    let response_text = response
        .text()
        .map_err(|e| {
            let msg = format!("Failed to read response body. HTTP status: {}. Error kind: {:?}. Error: {}", status, e, e);
            error(&msg);
            msg
        })?;

    if !status.is_success() {
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
            return Err(format!("API Error: {}", error_response.error.message));
        } else {
            return Err(format!("HTTP Error {}: {}", status, response_text));
        }
    }

    let chat_response: ChatResponse = serde_json::from_str(&response_text)
        .map_err(|e| {
            let msg = format!("Failed to parse JSON response: {}. Raw response: {}", e, response_text);
            error(&msg);
            msg
        })?;

    // Check for API-level errors in the response
    if let Some(error) = chat_response.error {
        return Err(format!("API Error: {}", error.message));
    }

    let response_text = chat_response
        .choices
        .first()
        .map(|choice| choice.message.content.clone())
        .ok_or_else(|| "No response content received".to_string())?;

    // Apply cleaning logic to the response
    Ok(clean_prompt(&response_text))
}

// Structures for OpenRouter API
#[derive(Serialize, Deserialize)]
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
