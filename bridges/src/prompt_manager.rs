use std::thread;
use core::pin::Pin;

use cxx_qt_lib::QString;
use cxx_qt::Threading;


use serde::{Deserialize, Serialize};
use rig::{completion::Prompt, completion::request::Chat, providers::deepseek, providers::gemini, providers::xai, providers::openrouter, providers::anthropic, providers::openai, client::CompletionClient, message::Message};
use rig::providers::gemini::completion::gemini_api_types::{AdditionalParameters, GenerationConfig};
use tokio::runtime::Runtime;

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

// Helper function to extract API keys
fn get_api_key(key_name: &str) -> String {
    std::env::var(key_name).unwrap_or_else(|_| {
        let app_data = get_app_data();
        app_data.get_api_key(key_name)
    })
}

// Helper function to create HTTP client with timeout for async operations
fn create_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}



// Helper function to convert ChatMessage to rig-core Message
fn convert_chat_messages_to_rig_messages(messages: &[ChatMessage]) -> Vec<Message> {
    messages.iter().map(|msg| {
        match msg.role.as_str() {
            "user" => Message::user(&msg.content),
            "assistant" => Message::assistant(&msg.content),
            _ => Message::user(&msg.content), // Default to user for unknown roles
        }
    }).collect()
}

impl qobject::PromptManager {
    fn prompt_request(self: Pin<&mut Self>, paragraph_idx: usize, translation_idx: usize, model_name: &QString, prompt: &QString) {
        let qt_thread = self.qt_thread();

        let prompt_text = prompt.to_string();
        let model_name_text = model_name.to_string();

        // Spawn a thread so Qt event loop is not blocked
        thread::spawn(move || {
            // Create a single message for the chat request
            let single_message = vec![ChatMessage {
                role: "user".to_string(),
                content: prompt_text,
            }];

            let response_content = if model_name_text == "deepseek-chat" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_deepseek_api_request(&single_message, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "gemini-2.5-flash" || model_name_text == "gemini-2.5-pro" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_gemini_api_request(&single_message, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "grok-4" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_xai_api_request(&single_message, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "claude-sonnet-4-0" || model_name_text == "claude-opus-4-0" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_anthropic_api_request(&single_message, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "gpt-4" || model_name_text == "gpt-4o" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_openai_api_request(&single_message, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_openrouter_api_request(&single_message, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
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
            let response_content = if model_name_text == "deepseek-chat" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_deepseek_api_request(&messages, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "gemini-2.5-flash" || model_name_text == "gemini-2.5-pro" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_gemini_api_request(&messages, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "grok-4" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_xai_api_request(&messages, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "claude-sonnet-4-0" || model_name_text == "claude-opus-4-0" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_anthropic_api_request(&messages, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else if model_name_text == "gpt-4" || model_name_text == "gpt-4o" {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_openai_api_request(&messages, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
            } else {
                let rt = Runtime::new().unwrap();
                match rt.block_on(make_openrouter_api_request(&messages, &model_name_text)) {
                    Ok(content) => content,
                    Err(e) => format!("Error: {}", e),
                }
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

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

async fn make_openrouter_api_request(messages: &[ChatMessage], model: &str) -> Result<String, String> {
    let api_key = get_api_key("OPENROUTER_API_KEY");

    let http_client = create_http_client()?;

    let client = openrouter::Client::builder(&api_key)
        .custom_client(http_client)
        .build()
        .map_err(|e| format!("Failed to build OpenRouter client: {}", e))?;

    let agent = client.agent(model)
        .temperature(0.7)
        .build();

    let response = if messages.len() == 1 {
        // Single message - handle as prompt
        let prompt_content = &messages[0].content;
        agent
            .prompt(prompt_content)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    } else {
        // Multiple messages - handle as chat
        // Convert ChatMessage to rig-core Messages
        let rig_messages = convert_chat_messages_to_rig_messages(messages);

        let (chat_history, current_prompt) = if let Some((last, rest)) = rig_messages.split_last() {
            (rest.to_vec(), last.clone())
        } else {
            return Err("No messages provided".to_string());
        };

        agent
            .chat(current_prompt, chat_history)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    };

    Ok(clean_prompt(&response))
}

async fn make_deepseek_api_request(messages: &[ChatMessage], model: &str) -> Result<String, String> {
    let api_key = get_api_key("DEEPSEEK_API_KEY");

    let http_client = create_http_client()?;

    let client = deepseek::Client::builder(&api_key)
        .custom_client(http_client)
        .build()
        .map_err(|e| format!("Failed to build DeepSeek client: {}", e))?;

    let agent = client.agent(model)
        .temperature(0.7)
        .build();

    let response = if messages.len() == 1 {
        // Single message - handle as prompt
        let prompt_content = &messages[0].content;
        agent
            .prompt(prompt_content)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    } else {
        // Multiple messages - handle as chat
        // Convert ChatMessage to rig-core Messages
        let rig_messages = convert_chat_messages_to_rig_messages(messages);

        let (chat_history, current_prompt) = if let Some((last, rest)) = rig_messages.split_last() {
            (rest.to_vec(), last.clone())
        } else {
            return Err("No messages provided".to_string());
        };

        agent
            .chat(current_prompt, chat_history)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    };

    Ok(clean_prompt(&response))
}

async fn make_gemini_api_request(messages: &[ChatMessage], model: &str) -> Result<String, String> {
    let api_key = get_api_key("GEMINI_API_KEY");

    let http_client = create_http_client()?;

    let client = gemini::Client::builder(&api_key)
        .custom_client(http_client)
        .build()
        .map_err(|e| format!("Failed to build Gemini client: {}", e))?;

    let gen_cfg = GenerationConfig {
        top_k: Some(1),
        top_p: Some(0.95),
        candidate_count: Some(1),
        max_output_tokens: Some(4096),
        ..Default::default()
    };
    let cfg = AdditionalParameters::default().with_config(gen_cfg);

    let agent = client
        .agent(model)
        .temperature(0.7)
        .additional_params(serde_json::to_value(cfg).map_err(|e| format!("Failed to serialize config: {}", e))?)
        .build();

    let response = if messages.len() == 1 {
        // Single message - handle as prompt
        let prompt_content = &messages[0].content;
        agent
            .prompt(prompt_content)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    } else {
        // Multiple messages - handle as chat
        // Convert ChatMessage to rig-core Messages
        let rig_messages = convert_chat_messages_to_rig_messages(messages);

        let (chat_history, current_prompt) = if let Some((last, rest)) = rig_messages.split_last() {
            (rest.to_vec(), last.clone())
        } else {
            return Err("No messages provided".to_string());
        };

        agent
            .chat(current_prompt, chat_history)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    };

    Ok(clean_prompt(&response))
}

async fn make_xai_api_request(messages: &[ChatMessage], model: &str) -> Result<String, String> {
    let api_key = get_api_key("XAI_API_KEY");

    let http_client = create_http_client()?;

    let client = xai::Client::builder(&api_key)
        .custom_client(http_client)
        .build()
        .map_err(|e| format!("Failed to build xAI client: {}", e))?;

    let agent = client
        .agent(model)
        .temperature(0.7)
        .build();

    let response = if messages.len() == 1 {
        // Single message - handle as prompt
        let prompt_content = &messages[0].content;
        agent
            .prompt(prompt_content)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    } else {
        // Multiple messages - handle as chat
        // Convert ChatMessage to rig-core Messages
        let rig_messages = convert_chat_messages_to_rig_messages(messages);

        let (chat_history, current_prompt) = if let Some((last, rest)) = rig_messages.split_last() {
            (rest.to_vec(), last.clone())
        } else {
            return Err("No messages provided".to_string());
        };

        agent
            .chat(current_prompt, chat_history)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    };

    Ok(clean_prompt(&response))
}

async fn make_anthropic_api_request(messages: &[ChatMessage], model: &str) -> Result<String, String> {
    let api_key = get_api_key("ANTHROPIC_API_KEY");

    let http_client = create_http_client()?;

    let client = anthropic::Client::builder(&api_key)
        .custom_client(http_client)
        .build()
        .map_err(|e| format!("Failed to build Anthropic client: {}", e))?;

    let agent = client.agent(model)
        .temperature(0.7)
        .build();

    let response = if messages.len() == 1 {
        // Single message - handle as prompt
        let prompt_content = &messages[0].content;
        agent
            .prompt(prompt_content)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    } else {
        // Multiple messages - handle as chat
        // Convert ChatMessage to rig-core Messages
        let rig_messages = convert_chat_messages_to_rig_messages(messages);

        let (chat_history, current_prompt) = if let Some((last, rest)) = rig_messages.split_last() {
            (rest.to_vec(), last.clone())
        } else {
            return Err("No messages provided".to_string());
        };

        agent
            .chat(current_prompt, chat_history)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    };

    Ok(clean_prompt(&response))
}

async fn make_openai_api_request(messages: &[ChatMessage], model: &str) -> Result<String, String> {
    let api_key = get_api_key("OPENAI_API_KEY");

    let http_client = create_http_client()?;

    let client = openai::Client::builder(&api_key)
        .custom_client(http_client)
        .build()
        .map_err(|e| format!("Failed to build OpenAI client: {}", e))?;

    let agent = client.agent(model)
        .temperature(0.7)
        .build();

    let response = if messages.len() == 1 {
        // Single message - handle as prompt
        let prompt_content = &messages[0].content;
        agent
            .prompt(prompt_content)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    } else {
        // Multiple messages - handle as chat
        // Convert ChatMessage to rig-core Messages
        let rig_messages = convert_chat_messages_to_rig_messages(messages);

        let (chat_history, current_prompt) = if let Some((last, rest)) = rig_messages.split_last() {
            (rest.to_vec(), last.clone())
        } else {
            return Err("No messages provided".to_string());
        };

        agent
            .chat(current_prompt, chat_history)
            .await
            .map_err(|e| format!("Failed to prompt {}: {}", model, e))?
    };

    Ok(clean_prompt(&response))
}
