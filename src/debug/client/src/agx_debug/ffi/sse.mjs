export function subscribe_sse(url, on_message) {
  const source = new EventSource(url);
  // TODO: error should be handled here
  source.onmessage = (event) => on_message(event.data);
}
