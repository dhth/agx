import agx_debug/effects
import agx_debug/model.{init_model}
import agx_debug/types.{type Model, type Msg}
import agx_debug/update
import agx_debug/view
import lustre
import lustre/effect

pub fn main() {
  let app = lustre.application(init, update.update, view.view)
  let assert Ok(_) = lustre.start(app, "#app", Nil)
}

fn init(_) -> #(Model, effect.Effect(Msg)) {
  #(
    init_model(),
    effects.subscribe_sse("http://127.0.0.1:4880/api/debug/events"),
  )
}
