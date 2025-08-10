import QtQuick

Item {
    function prompt_request(paragraph_idx: int, translation_idx: int, model: string, prompt: string,) {
        console.log("prompt_request():", paragraph_idx, translation_idx, model, prompt.slice(0, 30));
    }

    signal promptResponse(paragraph_idx: int, translation_idx: int, model: string, response: string);
}
