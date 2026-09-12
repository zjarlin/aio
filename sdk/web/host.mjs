export function mountBridge(frame, invoke, options = {}) {
  let active = true;
  let inflight = 0;
  const listener = async (event) => {
    const message = event.data;
    if (!active || event.source !== frame.contentWindow || event.origin !== "null" || message?.protocol !== "aio:plugin@2" || !["request", "clipboard"].includes(message.kind)) return;
    if (typeof message.id !== "string" || message.id.length > 80) return;
    const source = event.source;
    const reply = { protocol: "aio:plugin@2", kind: "response", id: message.id };
    try {
      if (inflight >= 16) throw new Error("Too many pending requests");
      inflight++;
      try {
        if (message.kind === "clipboard") {
          if (!options.clipboard || !document.hasFocus() || !navigator.userActivation?.isActive || typeof message.text !== "string" || message.text.length > 100000) throw new Error("Clipboard access denied");
          await navigator.clipboard.writeText(message.text);
          reply.response = { status: 204, headers: [], body: [] };
        } else reply.response = await invoke(message.request);
      }
      finally { inflight--; }
    } catch (cause) { reply.error = String(cause.message ?? cause); }
    if (active && source === frame.contentWindow) source.postMessage(reply, "*");
  };
  window.addEventListener("message", listener);
  return () => { active = false; window.removeEventListener("message", listener); };
}
