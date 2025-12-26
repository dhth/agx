import agx_debug/types.{type Msg, EventReceived, decode_event}
import lustre/effect

@external(javascript, "./ffi/sse.mjs", "subscribe_sse")
fn subscribe_sse_js(url: String, on_message: fn(String) -> Nil) -> Nil

@external(javascript, "./ffi/scroll.mjs", "scroll_to_bottom")
fn scroll_to_bottom_js() -> Nil

@external(javascript, "./ffi/scroll.mjs", "scroll_to_element")
fn scroll_to_element_js(id: String) -> Nil

pub fn subscribe_sse(url: String) -> effect.Effect(Msg) {
  effect.from(fn(dispatch) {
    subscribe_sse_js(url, fn(raw_json) {
      let event = decode_event(raw_json)
      dispatch(EventReceived(event))
    })
  })
}

pub fn scroll_to_bottom() -> effect.Effect(Msg) {
  effect.from(fn(_) { scroll_to_bottom_js() })
}

pub fn scroll_to_element(id: String) -> effect.Effect(Msg) {
  effect.from(fn(_) { scroll_to_element_js(id) })
}
