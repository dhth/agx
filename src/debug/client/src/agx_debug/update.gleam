import agx_debug/types.{type Model, type Msg, Model}
import lustre/effect

pub fn update(model: Model, msg: Msg) -> #(Model, effect.Effect(Msg)) {
  let zero = #(model, effect.none())

  case msg {
    types.EventReceived(result) ->
      case result {
        Ok(event) -> #(Model(events: [event, ..model.events]), effect.none())
        Error(_) -> zero
      }
  }
}
