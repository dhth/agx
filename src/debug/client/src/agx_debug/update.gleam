import agx_debug/effects
import agx_debug/types.{type Model, type Msg, Controls, Model}
import gleam/int
import lustre/effect

pub fn update(model: Model, msg: Msg) -> #(Model, effect.Effect(Msg)) {
  let zero = #(model, effect.none())

  case msg {
    types.EventReceived(result) ->
      case result {
        Ok(event) -> {
          let new_model = Model(..model, events: [event, ..model.events])
          let eff = case model.controls.scroll_to_new_event {
            True -> effects.scroll_to_bottom()
            False -> effect.none()
          }
          #(new_model, eff)
        }
        Error(_) -> zero
      }

    types.ToggleScrollToNewEvent -> {
      let new_controls =
        Controls(scroll_to_new_event: !model.controls.scroll_to_new_event)
      #(Model(..model, controls: new_controls), effect.none())
    }

    types.ScrollToEvent(index) -> {
      let id = "event-" <> int.to_string(index)
      #(model, effects.scroll_to_element(id))
    }
  }
}
