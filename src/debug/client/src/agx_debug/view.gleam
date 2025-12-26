import agx_debug/types.{type DebugEvent, type Model, type Msg, DebugEvent}
import gleam/list
import lustre/attribute
import lustre/element
import lustre/element/html

pub fn view(model: Model) -> element.Element(Msg) {
  html.div(
    [
      attribute.class(
        "dark:bg-slate-800 dark:text-slate-100 text-slate-800 select-none",
      ),
    ],
    list.map(model.events, render_event),
  )
}

fn render_event(event: DebugEvent) -> element.Element(Msg) {
  let DebugEvent(raw) = event
  html.pre(
    [attribute.class("p-2 m-2 bg-slate-700 rounded text-sm overflow-x-auto")],
    [html.text(raw)],
  )
}
