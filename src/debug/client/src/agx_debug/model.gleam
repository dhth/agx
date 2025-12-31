import agx_debug/types.{type Controls, type Model, Controls, Model}

pub fn init_model() -> Model {
  Model(events: [], controls: default_controls())
}

fn default_controls() -> Controls {
  Controls(scroll_to_new_event: False)
}
