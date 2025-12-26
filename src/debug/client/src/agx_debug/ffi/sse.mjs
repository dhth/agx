export function subscribe_sse(url, on_message) {
  const source = new EventSource(url);
  source.onmessage = (event) => on_message(event.data);
}
