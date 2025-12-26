import agx_debug/types.{type DebugEvent, type Model, type Msg, DebugEvent}
import gleam/list
import lustre/attribute
import lustre/element
import lustre/element/html

pub fn view(model: Model) -> element.Element(Msg) {
  html.div(
    [attribute.class("flex flex-col h-screen bg-[#282828] text-[#ebdbb2]")],
    [
      html.div([attribute.class("mt-8 w-4/5 mx-auto")], [
        html.div([], [
          heading(),
          events_div(model.events),
        ]),
      ]),
    ],
  )
}

fn heading() -> element.Element(Msg) {
  html.h1([attribute.class("text-3xl font-bold mb-4")], [
    html.a(
      [
        attribute.href("https://github.com/dhth/agx"),
        attribute.target("_blank"),
      ],
      [element.text("agx debug")],
    ),
  ])
}

fn events_div(events: List(DebugEvent)) -> element.Element(Msg) {
  html.div([attribute.class("flex flex-col gap-1")], case events {
    [] -> [element.text("no events")]
    events -> list.map(events, render_event)
  })
}

fn render_event(event: DebugEvent) -> element.Element(Msg) {
  let DebugEvent(raw) = event
  html.pre(
    [
      attribute.class(
        "p-2 m-2 bg-[#a89984] text-[#282828] rounded text-sm overflow-x-scroll overflow-y-auto",
      ),
    ],
    [html.text(raw)],
  )
}
