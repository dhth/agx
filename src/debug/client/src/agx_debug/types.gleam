import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode.{type Decoder}
import gleam/json
import gleam/option.{type Option}

pub type Controls {
  Controls(scroll_to_new_event: Bool)
}

pub type Model {
  Model(events: List(DebugEvent), controls: Controls)
}

pub type Msg {
  EventReceived(Result(DebugEvent, json.DecodeError))
  ToggleScrollToNewEvent
  ScrollToEvent(Int)
}

pub type DebugEvent {
  DebugEvent(timestamp: String, payload: DebugEventPayload)
}

pub type DebugEventPayload {
  LlmRequest(prompt: Message, history: String)
  AssistantTextEvent(text: String)
  ToolCallEvent(tool_call: ToolCallData)
  ReasoningEvent(reasoning: ReasoningData)
  ToolResultEvent(id: String, call_id: Option(String), content: String)
  StreamComplete
  TurnComplete(history: String)
  Interrupted
  NewSession
}

pub type ToolCallData {
  ToolCallData(
    id: String,
    call_id: option.Option(String),
    function: ToolFunction,
    signature: option.Option(String),
  )
}

pub type ReasoningData {
  ReasoningData(
    id: option.Option(String),
    reasoning: List(String),
    signature: option.Option(String),
  )
}

pub type Message {
  UserMessage(content: List(UserContent))
  AssistantMessage(id: Option(String), content: List(AssistantContent))
}

pub type UserContent {
  UserText(text: String)
  ToolResult(id: String, call_id: Option(String), content: String)
  UnsupportedUserContent(raw: String)
}

pub type AssistantContent {
  AssistantText(text: String)
  ToolCall(
    id: String,
    call_id: Option(String),
    function: ToolFunction,
    signature: Option(String),
  )
  Reasoning(
    id: Option(String),
    reasoning: List(String),
    signature: Option(String),
  )
  UnsupportedAssistantContent(raw: String)
}

pub type ToolFunction {
  ToolFunction(name: String, arguments: String)
}

pub fn decode_event(raw_json: String) -> Result(DebugEvent, json.DecodeError) {
  json.parse(raw_json, debug_event_decoder())
}

fn debug_event_decoder() -> Decoder(DebugEvent) {
  use timestamp <- decode.field("timestamp", decode.string)
  use payload <- decode.field("payload", debug_event_payload_decoder())
  decode.success(DebugEvent(timestamp:, payload:))
}

fn debug_event_payload_decoder() -> Decoder(DebugEventPayload) {
  use kind <- decode.field("kind", decode.string)
  case kind {
    "llm_request" -> llm_request_payload_decoder()
    "assistant_text" -> assistant_text_payload_decoder()
    "tool_call" -> tool_call_payload_decoder()
    "reasoning" -> reasoning_payload_decoder()
    "tool_result" -> tool_result_event_payload_decoder()
    "stream_complete" -> stream_complete_payload_decoder()
    "turn_complete" -> turn_complete_payload_decoder()
    "interrupted" -> interrupted_payload_decoder()
    "new_session" -> new_session_payload_decoder()
    _ -> decode.failure(LlmRequest(UserMessage([]), ""), "unknown payload kind")
  }
}

fn llm_request_payload_decoder() -> Decoder(DebugEventPayload) {
  use prompt <- decode.field("prompt", message_decoder())
  use history <- decode.field("history", raw_json_decoder())
  decode.success(LlmRequest(prompt:, history:))
}

fn assistant_text_payload_decoder() -> Decoder(DebugEventPayload) {
  use text <- decode.field("text", decode.string)
  decode.success(AssistantTextEvent(text:))
}

fn tool_call_payload_decoder() -> Decoder(DebugEventPayload) {
  use tool_call <- decode.field("tool_call", tool_call_data_decoder())
  decode.success(ToolCallEvent(tool_call:))
}

fn reasoning_payload_decoder() -> Decoder(DebugEventPayload) {
  use reasoning <- decode.field("reasoning", reasoning_data_decoder())
  decode.success(ReasoningEvent(reasoning:))
}

fn stream_complete_payload_decoder() -> Decoder(DebugEventPayload) {
  decode.success(StreamComplete)
}

fn turn_complete_payload_decoder() -> Decoder(DebugEventPayload) {
  use history <- decode.field("history", raw_json_decoder())
  decode.success(TurnComplete(history:))
}

fn interrupted_payload_decoder() -> Decoder(DebugEventPayload) {
  decode.success(Interrupted)
}

fn new_session_payload_decoder() -> Decoder(DebugEventPayload) {
  decode.success(NewSession)
}

fn tool_result_event_payload_decoder() -> Decoder(DebugEventPayload) {
  use id <- decode.field("id", decode.string)
  use call_id <- decode.optional_field(
    "call_id",
    option.None,
    decode.optional(decode.string),
  )
  use content <- decode.field("content", raw_json_decoder())
  decode.success(ToolResultEvent(id:, call_id:, content:))
}

fn tool_call_data_decoder() -> Decoder(ToolCallData) {
  use id <- decode.field("id", decode.string)
  use call_id <- decode.optional_field(
    "call_id",
    option.None,
    decode.optional(decode.string),
  )
  use function <- decode.field("function", tool_function_decoder())
  use signature <- decode.optional_field(
    "signature",
    option.None,
    decode.optional(decode.string),
  )
  decode.success(ToolCallData(id:, call_id:, function:, signature:))
}

fn reasoning_data_decoder() -> Decoder(ReasoningData) {
  use id <- decode.optional_field(
    "id",
    option.None,
    decode.optional(decode.string),
  )
  use reasoning <- decode.field("reasoning", decode.list(decode.string))
  use signature <- decode.optional_field(
    "signature",
    option.None,
    decode.optional(decode.string),
  )
  decode.success(ReasoningData(id:, reasoning:, signature:))
}

fn message_decoder() -> Decoder(Message) {
  use role <- decode.field("role", decode.string)
  case role {
    "user" -> user_message_decoder()
    "assistant" -> assistant_message_decoder()
    _ -> decode.failure(UserMessage([]), "unknown message role")
  }
}

fn user_message_decoder() -> Decoder(Message) {
  use content <- decode.field("content", decode.list(user_content_decoder()))
  decode.success(UserMessage(content:))
}

fn assistant_message_decoder() -> Decoder(Message) {
  use id <- decode.optional_field(
    "id",
    option.None,
    decode.optional(decode.string),
  )
  use content <- decode.field(
    "content",
    decode.list(assistant_content_decoder()),
  )
  decode.success(AssistantMessage(id:, content:))
}

fn user_content_decoder() -> Decoder(UserContent) {
  use type_field <- decode.field("type", decode.string)
  case type_field {
    "text" -> user_text_decoder()
    "toolresult" -> tool_result_decoder()
    _ -> raw_json_decoder() |> decode.map(UnsupportedUserContent)
  }
}

fn user_text_decoder() -> Decoder(UserContent) {
  use text <- decode.field("text", decode.string)
  decode.success(UserText(text:))
}

fn tool_result_decoder() -> Decoder(UserContent) {
  use id <- decode.field("id", decode.string)
  use call_id <- decode.optional_field(
    "call_id",
    option.None,
    decode.optional(decode.string),
  )
  use content <- decode.field("content", raw_json_decoder())
  decode.success(ToolResult(id:, call_id:, content:))
}

fn assistant_content_decoder() -> Decoder(AssistantContent) {
  decode.one_of(assistant_text_decoder(), [
    tool_call_decoder(),
    reasoning_decoder(),
    raw_json_decoder() |> decode.map(UnsupportedAssistantContent),
  ])
}

fn assistant_text_decoder() -> Decoder(AssistantContent) {
  use text <- decode.field("text", decode.string)
  decode.success(AssistantText(text:))
}

fn tool_call_decoder() -> Decoder(AssistantContent) {
  use id <- decode.field("id", decode.string)
  use call_id <- decode.optional_field(
    "call_id",
    option.None,
    decode.optional(decode.string),
  )
  use function <- decode.field("function", tool_function_decoder())
  use signature <- decode.optional_field(
    "signature",
    option.None,
    decode.optional(decode.string),
  )
  decode.success(ToolCall(id:, call_id:, function:, signature:))
}

fn tool_function_decoder() -> Decoder(ToolFunction) {
  use name <- decode.field("name", decode.string)
  use arguments <- decode.field("arguments", raw_json_decoder())
  decode.success(ToolFunction(name:, arguments:))
}

fn reasoning_decoder() -> Decoder(AssistantContent) {
  use id <- decode.optional_field(
    "id",
    option.None,
    decode.optional(decode.string),
  )
  use reasoning <- decode.field("reasoning", decode.list(decode.string))
  use signature <- decode.optional_field(
    "signature",
    option.None,
    decode.optional(decode.string),
  )
  decode.success(Reasoning(id:, reasoning:, signature:))
}

fn raw_json_decoder() -> Decoder(String) {
  decode.dynamic
  |> decode.map(stringify_dynamic)
}

@external(javascript, "./ffi/json.mjs", "stringify")
fn stringify_dynamic(value: Dynamic) -> String
