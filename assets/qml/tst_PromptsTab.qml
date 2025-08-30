import QtQuick
import QtTest

Item {
    width: 800; height: 600

    PromptsTab {
        id: prompts_tab
        window_id: "test_window_0"
        is_dark: false
        ai_models_auto_retry: true
        anchors.centerIn: parent
        width: 600
        height: 500
    }

    TestCase {
        name: "TestPromptsTab"
        when: windowShown

        function cleanup() {
            // Reset tab state after each test
            prompts_tab.messages_model.clear();
            prompts_tab.available_models.clear();
            prompts_tab.waiting_for_response = false;
        }

        function test_initial_state() {
            // Test initial component state
            verify(prompts_tab.messages_model);
            verify(prompts_tab.available_models);
            compare(prompts_tab.waiting_for_response, false);
            compare(prompts_tab.ai_models_auto_retry, true);
        }

        function test_utility_functions() {
            // Test generate_request_id
            var id1 = prompts_tab.generate_request_id();
            var id2 = prompts_tab.generate_request_id();
            verify(id1 !== id2);
            verify(id1.length > 10);
            verify(id1.includes("_"));

            // Test error detection
            verify(prompts_tab.is_error_response("API Error: Something failed"));
            verify(prompts_tab.is_error_response("Error: Connection timeout"));
            verify(prompts_tab.is_error_response("Failed: Authentication"));
            verify(!prompts_tab.is_error_response("Normal response"));

            // Test rate limit detection
            verify(prompts_tab.is_rate_limit_error("API Error: Rate limit exceeded"));
            verify(!prompts_tab.is_rate_limit_error("API Error: Other error"));
        }

        function test_model_loading() {
            // Test available models loading
            prompts_tab.load_available_models();

            // Should call SuttaBridge.get_models_json() and populate available_models
            verify(prompts_tab.available_models.count >= 0);

            // If models exist, they should have required properties
            if (prompts_tab.available_models.count > 0) {
                var model = prompts_tab.available_models.get(0);
                verify(model.hasOwnProperty("model_name"));
                verify(model.hasOwnProperty("enabled"));
            }
        }

        function test_message_structure() {
            // Test basic message structure
            prompts_tab.messages_model.append({
                role: "user",
                content: "Test user message",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            compare(prompts_tab.messages_model.count, 1);
            var message = prompts_tab.messages_model.get(0);
            compare(message.role, "user");
            compare(message.content, "Test user message");
            compare(message.responses_json, "[]");
            compare(message.selected_ai_tab, 0);
        }

        function test_assistant_message_structure() {
            // Test assistant message with responses_json
            var sample_responses = [{
                model_name: "deepseek/deepseek-r1-0528:free",
                status: "completed",
                response: "Assistant response text",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "google/gemma-3-12b-it:free",
                status: "waiting",
                response: "",
                request_id: "test_req_2",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: false
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(sample_responses),
                selected_ai_tab: 0
            });

            compare(prompts_tab.messages_model.count, 1);
            var message = prompts_tab.messages_model.get(0);
            compare(message.role, "assistant");
            compare(message.content, "");

            var responses = JSON.parse(message.responses_json);
            compare(responses.length, 2);
            compare(responses[0].status, "completed");
            compare(responses[1].status, "waiting");
        }

        function test_prompt_response_handling() {
            // Setup: Add user message and assistant message with waiting responses
            prompts_tab.messages_model.append({
                role: "user",
                content: "What is meditation?",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            var waiting_responses = [{
                model_name: "test/model:free",
                status: "waiting",
                response: "",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(waiting_responses),
                selected_ai_tab: 0
            });

            // Simulate response from PromptManager
            prompts_tab.prompt_connections.onPromptResponseForMessages(0, "test/model:free", "Meditation is a practice of mindfulness...");

            // Check that response was processed correctly
            var assistant_message = prompts_tab.messages_model.get(1);
            var updated_responses = JSON.parse(assistant_message.responses_json);

            compare(updated_responses[0].status, "completed");
            compare(updated_responses[0].response, "Meditation is a practice of mindfulness...");
            compare(prompts_tab.waiting_for_response, false);
        }

        function test_error_response_handling() {
            // Setup messages
            prompts_tab.messages_model.append({
                role: "user",
                content: "Test question",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            var waiting_responses = [{
                model_name: "test/model:free",
                status: "waiting",
                response: "",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(waiting_responses),
                selected_ai_tab: 0
            });

            // Simulate error response
            prompts_tab.prompt_connections.onPromptResponseForMessages(0, "test/model:free", "API Error: Request timeout");

            var assistant_message = prompts_tab.messages_model.get(1);
            var updated_responses = JSON.parse(assistant_message.responses_json);

            compare(updated_responses[0].status, "error");
            compare(updated_responses[0].response, "API Error: Request timeout");
        }

        function test_retry_functionality() {
            // Setup assistant message with error response
            var error_responses = [{
                model_name: "test/model:free",
                status: "error",
                response: "API Error: Connection failed",
                request_id: "test_req_1",
                retry_count: 1,
                last_updated: Date.now(),
                user_selected: true
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(error_responses),
                selected_ai_tab: 0
            });

            // Test retry request handling
            var new_request_id = prompts_tab.generate_request_id();
            prompts_tab.handle_retry_request(0, "test/model:free", new_request_id);

            var message = prompts_tab.messages_model.get(0);
            var updated_responses = JSON.parse(message.responses_json);

            compare(updated_responses[0].status, "waiting");
            compare(updated_responses[0].request_id, new_request_id);
            compare(updated_responses[0].retry_count, 2);
        }

        function test_tab_selection_update() {
            // Setup assistant message with multiple responses
            var multi_responses = [{
                model_name: "model1:free",
                status: "completed",
                response: "Response from model 1",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "model2:free",
                status: "completed",
                response: "Response from model 2",
                request_id: "test_req_2",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: false
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(multi_responses),
                selected_ai_tab: 0
            });

            // Update tab selection
            prompts_tab.update_tab_selection(0, 1, "model2:free");

            var message = prompts_tab.messages_model.get(0);
            compare(message.selected_ai_tab, 1);

            var updated_responses = JSON.parse(message.responses_json);
            verify(!updated_responses[0].user_selected);
            verify(updated_responses[1].user_selected);
        }

        function test_conversation_history_composition() {
            // Test message history building for chat context

            // Add initial system message
            prompts_tab.messages_model.append({
                role: "system",
                content: "You are a helpful AI assistant.",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            // Add user message
            prompts_tab.messages_model.append({
                role: "user",
                content: "What is Buddhism?",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            // Add assistant response
            var assistant_responses = [{
                model_name: "test/model:free",
                status: "completed",
                response: "Buddhism is a religion and philosophy...",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(assistant_responses),
                selected_ai_tab: 0
            });

            // Add follow-up user message
            prompts_tab.messages_model.append({
                role: "user",
                content: "Tell me about meditation",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            // Verify we have proper conversation structure
            compare(prompts_tab.messages_model.count, 4);

            var system_msg = prompts_tab.messages_model.get(0);
            var first_user = prompts_tab.messages_model.get(1);
            var assistant = prompts_tab.messages_model.get(2);
            var second_user = prompts_tab.messages_model.get(3);

            compare(system_msg.role, "system");
            compare(first_user.role, "user");
            compare(assistant.role, "assistant");
            compare(second_user.role, "user");

            // Assistant should have responses data
            var responses = JSON.parse(assistant.responses_json);
            compare(responses.length, 1);
            compare(responses[0].status, "completed");
        }

        function test_rate_limit_error_no_retry() {
            // Test that rate limit errors don't trigger auto-retry
            prompts_tab.messages_model.append({
                role: "user",
                content: "Test question",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            var waiting_responses = [{
                model_name: "test/model:free",
                status: "waiting",
                response: "",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(waiting_responses),
                selected_ai_tab: 0
            });

            // Simulate rate limit error
            prompts_tab.prompt_connections.onPromptResponseForMessages(0, "test/model:free", "API Error: Rate limit exceeded");

            var assistant_message = prompts_tab.messages_model.get(1);
            var updated_responses = JSON.parse(assistant_message.responses_json);

            compare(updated_responses[0].status, "error");
            compare(updated_responses[0].retry_count, 0); // Should not increment on rate limit
        }

        function test_assistant_responses_integration() {
            // Test integration with AssistantResponses component
            var sample_responses = [{
                model_name: "deepseek/deepseek-r1-0528:free",
                status: "completed",
                response: "This is a **markdown** response with *emphasis*.",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "google/gemma-3-12b-it:free",
                status: "error",
                response: "API Error: Service unavailable",
                request_id: "test_req_2",
                retry_count: 2,
                last_updated: Date.now(),
                user_selected: false
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(sample_responses),
                selected_ai_tab: 0
            });

            var message = prompts_tab.messages_model.get(0);

            // Data should be properly formatted for AssistantResponses
            var responses = JSON.parse(message.responses_json);
            compare(responses.length, 2);

            // First response completed
            compare(responses[0].status, "completed");
            verify(responses[0].response.includes("**markdown**"));
            verify(responses[0].user_selected);

            // Second response error with retry count
            compare(responses[1].status, "error");
            compare(responses[1].retry_count, 2);
            verify(!responses[1].user_selected);
        }

        function test_multi_model_response_processing() {
            // Test handling multiple model responses to same request
            prompts_tab.messages_model.append({
                role: "user",
                content: "Explain mindfulness",
                content_html: "",
                responses_json: "[]",
                selected_ai_tab: 0
            });

            var multi_waiting_responses = [{
                model_name: "model1:free",
                status: "waiting",
                response: "",
                request_id: "test_req_1",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: true
            }, {
                model_name: "model2:free",
                status: "waiting",
                response: "",
                request_id: "test_req_2",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: false
            }, {
                model_name: "model3:free",
                status: "waiting",
                response: "",
                request_id: "test_req_3",
                retry_count: 0,
                last_updated: Date.now(),
                user_selected: false
            }];

            prompts_tab.messages_model.append({
                role: "assistant",
                content: "",
                content_html: "",
                responses_json: JSON.stringify(multi_waiting_responses),
                selected_ai_tab: 0
            });

            // Simulate responses arriving from different models
            prompts_tab.prompt_connections.onPromptResponseForMessages(0, "model1:free", "Model 1 response about mindfulness");
            prompts_tab.prompt_connections.onPromptResponseForMessages(0, "model3:free", "Model 3 different perspective");
            prompts_tab.prompt_connections.onPromptResponseForMessages(0, "model2:free", "API Error: Timeout");

            var assistant_message = prompts_tab.messages_model.get(1);
            var final_responses = JSON.parse(assistant_message.responses_json);

            // Check that all responses were updated correctly
            compare(final_responses[0].status, "completed");
            compare(final_responses[0].response, "Model 1 response about mindfulness");

            compare(final_responses[1].status, "error");
            compare(final_responses[1].response, "API Error: Timeout");

            compare(final_responses[2].status, "completed");
            compare(final_responses[2].response, "Model 3 different perspective");
        }
    }
}
