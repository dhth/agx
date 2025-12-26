import gleam/json

pub type Model {
  Model(events: List(DebugEvent))
}

pub type DebugEvent {
  DebugEvent(String)
}

pub type Msg {
  EventReceived(Result(DebugEvent, json.DecodeError))
}

pub fn decode_event(raw_json: String) -> Result(DebugEvent, json.DecodeError) {
  Ok(DebugEvent(raw_json))
}
