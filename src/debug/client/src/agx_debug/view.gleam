import agx_debug/types.{
  type AssistantContent, type Controls, type DebugEvent, type DebugEventPayload,
  type Message, type Model, type Msg, type ReasoningData, type ToolCallData,
  type ToolFunction, type ToolResultContent, type Usage, type UserContent,
  AssistantMessage, AssistantText, AssistantTextEvent, DebugEvent, LlmRequest,
  Reasoning, ReasoningData, ReasoningEvent, ToggleScrollToNewEvent, ToolCall,
  ToolCallData, ToolCallEvent, ToolFunction, ToolResult, ToolResultText,
  TurnComplete, UnsupportedAssistantContent, UnsupportedToolResultContent,
  UnsupportedUserContent, Usage, UserMessage, UserText,
}
import gleam/int
import gleam/list
import gleam/option
import gleam/string
import lustre/attribute
import lustre/element
import lustre/element/html
import lustre/event

pub fn view(model: Model) -> element.Element(Msg) {
  html.div(
    [attribute.class("flex flex-col min-h-screen bg-[#282828] text-[#ebdbb2]")],
    [
      html.div([attribute.class("mt-8 mb-12 w-4/5 mx-auto")], [
        html.div([], [
          heading(),
          events_div(model.events),
        ]),
      ]),
      control_panel(model.controls),
    ],
  )
}

fn control_panel(controls: Controls) -> element.Element(Msg) {
  html.div(
    [
      attribute.class(
        "fixed bottom-0 left-0 right-0 h-8 bg-[#3c3836] border-t border-[#504945] flex items-center px-4 text-sm text-[#a89984] z-50",
      ),
    ],
    [
      html.label([attribute.class("flex items-center gap-2 cursor-pointer")], [
        html.input([
          attribute.type_("checkbox"),
          attribute.checked(controls.scroll_to_new_event),
          event.on_click(ToggleScrollToNewEvent),
          attribute.class("cursor-pointer"),
        ]),
        element.text("scroll to new event"),
      ]),
    ],
  )
}

fn heading() -> element.Element(Msg) {
  html.h1([attribute.class("font-bold")], [
    html.a(
      [
        attribute.href("https://github.com/dhth/agx"),
        attribute.target("_blank"),
      ],
      [
        html.span(
          [
            attribute.class("text-[#d3869b] text-4xl"),
          ],
          [
            element.text("agx"),
            html.sup(
              [
                attribute.class("text-[#a89984] text-base ml-1"),
              ],
              [element.text("[debug]")],
            ),
          ],
        ),
      ],
    ),
  ])
}

fn events_div(events: List(DebugEvent)) -> element.Element(Msg) {
  let count = list.length(events)
  let count_text = case count {
    0 -> "No events"
    _ -> string.append("Events: ", int.to_string(count))
  }

  html.div([attribute.class("mt-4")], [
    html.div([attribute.class("text-lg font-semibold mb-2")], [
      element.text(count_text),
    ]),
    html.div(
      [attribute.class("flex flex-col gap-4")],
      render_events(list.reverse(events), 0),
    ),
  ])
}

fn render_events(
  events: List(DebugEvent),
  start_index: Int,
) -> List(element.Element(Msg)) {
  case events {
    [] -> []
    [event, ..rest] -> [
      render_event_details(event, start_index),
      ..render_events(rest, start_index + 1)
    ]
  }
}

fn render_event_details(event: DebugEvent, index: Int) -> element.Element(Msg) {
  let DebugEvent(timestamp:, payload:) = event
  let #(kind, color) = payload_kind_and_color(payload)
  html.div([attribute.class("flex gap-3 items-start")], [
    html.div(
      [
        attribute.class(
          "flex-shrink-0 w-36 flex flex-col justify-between p-3 rounded text-sm font-mono",
        ),
        attribute.style("background-color", color),
      ],
      [
        html.div([attribute.class("font-semibold text-[#282828]")], [
          element.text(kind),
        ]),
        html.div(
          [
            attribute.class(
              "flex justify-between text-xs text-[#282828] opacity-70 mt-2",
            ),
          ],
          [
            html.span([], [element.text(int.to_string(index + 1))]),
            html.span([], [element.text(format_timestamp(timestamp))]),
          ],
        ),
      ],
    ),
    html.div([attribute.class("flex-1 flex flex-col gap-2 min-w-0")], [
      render_payload(payload),
    ]),
  ])
}

fn payload_kind_and_color(payload: DebugEventPayload) -> #(String, String) {
  case payload {
    LlmRequest(prompt: _, history: _) -> #("llm_request", "#fe8019")
    AssistantTextEvent(text: _) -> #("assistant_text", "#d5c4a1")
    ToolCallEvent(tool_call: _) -> #("tool_call", "#d3869b")
    ReasoningEvent(reasoning: _) -> #("reasoning", "#83a598")
    TurnComplete(usage: _) -> #("turn_complete", "#b8bb26")
  }
}

fn format_timestamp(timestamp: String) -> String {
  case string.split(timestamp, "T") {
    [_, time_part] -> {
      let without_z = string.replace(time_part, "Z", "")
      case string.split(without_z, ".") {
        [time, ..] -> time
        _ -> without_z
      }
    }
    _ -> timestamp
  }
}

fn render_payload(payload: DebugEventPayload) -> element.Element(Msg) {
  case payload {
    LlmRequest(prompt:, history:) ->
      html.div([attribute.class("flex flex-col gap-3")], [
        html.div([], [render_message(prompt)]),
        html.div([], [
          html.div(
            [attribute.class("text-sm font-semibold text-[#a89984] mb-1")],
            [
              element.text("History"),
            ],
          ),
          html.pre(
            [
              attribute.class(
                "p-2 bg-[#3c3836] text-[#ebdbb2] rounded text-xs whitespace-pre-wrap break-all max-h-[50vh] overflow-auto",
              ),
            ],
            [html.text(history)],
          ),
        ]),
      ])

    AssistantTextEvent(text:) ->
      html.div(
        [
          attribute.class(
            "p-2 bg-[#3c3836] rounded text-sm whitespace-pre-wrap",
          ),
        ],
        [element.text(text)],
      )

    ToolCallEvent(tool_call:) -> render_tool_call_data(tool_call)

    ReasoningEvent(reasoning:) -> render_reasoning_data(reasoning)

    TurnComplete(usage:) -> render_usage(usage)
  }
}

fn render_tool_call_data(tool_call: ToolCallData) -> element.Element(Msg) {
  let ToolCallData(id:, call_id: _, function:, signature: _) = tool_call
  let ToolFunction(name:, arguments:) = function
  html.div([attribute.class("p-2 bg-[#3c3836] rounded")], [
    html.div([attribute.class("flex gap-2 items-center mb-1")], [
      html.span(
        [attribute.class("font-mono text-sm bg-[#282828] px-1 rounded")],
        [
          element.text(name),
        ],
      ),
      html.span([attribute.class("text-xs text-[#a89984]")], [
        element.text("id: " <> id),
      ]),
    ]),
    html.pre(
      [
        attribute.class(
          "text-xs bg-[#282828] p-1 rounded whitespace-pre-wrap break-all",
        ),
      ],
      [
        html.text(arguments),
      ],
    ),
  ])
}

fn render_reasoning_data(reasoning: ReasoningData) -> element.Element(Msg) {
  let ReasoningData(id: _, reasoning: reasoning_list, signature: _) = reasoning
  html.div([attribute.class("p-2 bg-[#3c3836] rounded italic text-sm")], [
    element.text(string.join(reasoning_list, " ")),
  ])
}

fn render_usage(usage: Usage) -> element.Element(Msg) {
  let Usage(input_tokens:, output_tokens:, total_tokens:) = usage
  html.div([attribute.class("p-2 bg-[#3c3836] rounded")], [
    html.div([attribute.class("flex gap-4 text-sm")], [
      html.span([], [
        html.span([attribute.class("text-[#a89984]")], [
          element.text("input tokens: "),
        ]),
        element.text(int.to_string(input_tokens)),
      ]),
      html.span([], [
        html.span([attribute.class("text-[#a89984]")], [
          element.text("output tokens: "),
        ]),
        element.text(int.to_string(output_tokens)),
      ]),
      html.span([], [
        html.span([attribute.class("text-[#a89984]")], [
          element.text("total tokens: "),
        ]),
        element.text(int.to_string(total_tokens)),
      ]),
    ]),
  ])
}

fn render_message(message: Message) -> element.Element(Msg) {
  case message {
    UserMessage(content:) ->
      html.div([attribute.class("p-2 bg-[#3c3836] rounded")], [
        html.div(
          [attribute.class("text-xs font-semibold mb-1 text-[#a89984]")],
          [
            element.text("user"),
          ],
        ),
        html.div(
          [attribute.class("flex flex-col gap-1")],
          list.map(content, render_user_content),
        ),
      ])
    AssistantMessage(id:, content:) ->
      html.div([attribute.class("p-2 bg-[#b16286] rounded")], [
        html.div([attribute.class("text-xs font-semibold mb-1")], [
          element.text(case id {
            option.Some(i) -> "assistant (" <> i <> ")"
            option.None -> "assistant"
          }),
        ]),
        html.div(
          [attribute.class("flex flex-col gap-1")],
          list.map(content, render_assistant_content),
        ),
      ])
  }
}

fn render_user_content(content: UserContent) -> element.Element(Msg) {
  case content {
    UserText(text:) ->
      html.div([attribute.class("text-sm")], [element.text(text)])
    ToolResult(id:, call_id: _, content: inner) ->
      html.div([attribute.class("text-sm")], [
        html.span(
          [attribute.class("font-mono text-xs bg-[#282828] px-1 rounded")],
          [
            element.text("tool_result: " <> id),
          ],
        ),
        html.div(
          [attribute.class("mt-1")],
          list.map(inner, render_tool_result_content),
        ),
      ])
    UnsupportedUserContent(raw:) ->
      html.pre(
        [
          attribute.class(
            "text-xs bg-[#282828] p-1 rounded whitespace-pre-wrap break-all",
          ),
        ],
        [
          html.text(raw),
        ],
      )
  }
}

fn render_tool_result_content(
  content: ToolResultContent,
) -> element.Element(Msg) {
  case content {
    ToolResultText(text:) ->
      html.pre(
        [
          attribute.class(
            "text-xs bg-[#282828] p-1 rounded whitespace-pre-wrap break-all",
          ),
        ],
        [
          html.text(text),
        ],
      )
    UnsupportedToolResultContent(raw:) ->
      html.pre(
        [
          attribute.class(
            "text-xs bg-[#282828] p-1 rounded whitespace-pre-wrap break-all",
          ),
        ],
        [
          html.text(raw),
        ],
      )
  }
}

fn render_assistant_content(content: AssistantContent) -> element.Element(Msg) {
  case content {
    AssistantText(text:) ->
      html.div([attribute.class("text-sm whitespace-pre-wrap")], [
        element.text(text),
      ])
    ToolCall(id:, call_id: _, function:, signature: _) ->
      render_tool_call(id, function)
    Reasoning(id: _, reasoning:, signature: _) ->
      html.div([attribute.class("text-sm italic")], [
        html.span([attribute.class("font-semibold")], [
          element.text("Reasoning: "),
        ]),
        element.text(string.join(reasoning, " ")),
      ])
    UnsupportedAssistantContent(raw:) ->
      html.pre(
        [
          attribute.class(
            "text-xs bg-[#282828] p-1 rounded whitespace-pre-wrap break-all",
          ),
        ],
        [
          html.text(raw),
        ],
      )
  }
}

fn render_tool_call(id: String, function: ToolFunction) -> element.Element(Msg) {
  let ToolFunction(name:, arguments:) = function
  html.div([attribute.class("text-sm")], [
    html.div([attribute.class("flex gap-2 items-center")], [
      html.span(
        [attribute.class("font-mono text-xs bg-[#282828] px-1 rounded")],
        [
          element.text(name),
        ],
      ),
      html.span([attribute.class("text-xs text-[#a89984]")], [
        element.text("id: " <> id),
      ]),
    ]),
    html.pre(
      [
        attribute.class(
          "mt-1 text-xs bg-[#282828] p-1 rounded whitespace-pre-wrap break-all",
        ),
      ],
      [
        html.text(arguments),
      ],
    ),
  ])
}
