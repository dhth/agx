export function scroll_to_bottom() {
  setTimeout(() => {
    window.scrollTo({ top: document.body.scrollHeight, behavior: "smooth" });
  }, 100);
}
