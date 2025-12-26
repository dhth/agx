import agx_debug/types.{type Msg, EventReceived, decode_event}
import lustre/effect

@external(javascript, "./ffi/sse.mjs", "subscribe_sse")
fn subscribe_sse_js(url: String, on_message: fn(String) -> Nil) -> Nil

pub fn subscribe_sse(url: String) -> effect.Effect(Msg) {
  effect.from(fn(dispatch) {
    subscribe_sse_js(url, fn(raw_json) {
      let event = decode_event(raw_json)
      dispatch(EventReceived(event))
    })
  })
}
