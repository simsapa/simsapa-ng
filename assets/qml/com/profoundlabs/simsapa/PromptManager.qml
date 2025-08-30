import QtQuick

Item {
    function prompt_request(paragraph_idx: int, translation_idx: int, model: string, prompt: string,) {
        console.log("prompt_request():", paragraph_idx, translation_idx, model, prompt.slice(0, 30));
    }

    function prompt_request_with_messages(sender_message_idx: int, model: string, messages_json: string,) {
        console.log("prompt_request_messages():", sender_message_idx, model, messages_json);
    }

    signal promptResponse(paragraph_idx: int, translation_idx: int, model: string, response: string);

    signal promptResponseForMessages(message_idx: int, response: string);
}
