import type { SignalingMessage } from "../types/messages";

export class SignalingClient {
  private ws: WebSocket | null = null;
  private onMessage: ((msg: SignalingMessage) => void | Promise<void>) | null = null;

  connect(token: string): Promise<void> {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${protocol}//${window.location.host}/api/ws?token=${token}`;
    console.log(`[signaling] connecting to ${url}`);
    const ws = new WebSocket(url);
    this.ws = ws;

    ws.onmessage = (event) => {
      const msg: SignalingMessage = JSON.parse(event.data);
      if (msg.type !== "radio-event") {
        console.log("[signaling] received:", event.data);
      }
      this.onMessage?.(msg);
    };

    ws.onclose = (event) => {
      console.log(`[signaling] WebSocket closed: code=${event.code} reason=${event.reason}`);
    };

    return new Promise((resolve, reject) => {
      ws.addEventListener("open", () => {
        console.log("[signaling] WebSocket connected");
        resolve();
      }, { once: true });
      ws.addEventListener("error", (event) => {
        console.error("[signaling] WebSocket error:", event);
        reject(new Error("WebSocket connection failed"));
      }, { once: true });
    });
  }

  send(msg: SignalingMessage): void {
    console.log("[signaling] sending:", msg.type);
    this.ws?.send(JSON.stringify(msg));
  }

  setOnMessage(handler: (msg: SignalingMessage) => void | Promise<void>): void {
    this.onMessage = handler;
  }

  isOpen(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  close(): void {
    console.log("[signaling] closing connection");
    this.ws?.close();
    this.ws = null;
  }
}
