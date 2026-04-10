import type { SignalingMessage } from "../types/messages";

export class SignalingClient {
  private ws: WebSocket | null = null;
  private onMessage: ((msg: SignalingMessage) => void | Promise<void>) | null = null;

  connect(token: string): void {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${protocol}//${window.location.host}/api/ws?token=${token}`;
    console.log(`[signaling] connecting to ${url}`);
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      console.log("[signaling] WebSocket connected");
    };

    this.ws.onmessage = (event) => {
      const msg: SignalingMessage = JSON.parse(event.data);
      if (msg.type !== "radio-event") {
        console.log("[signaling] received:", event.data);
      }
      this.onMessage?.(msg);
    };

    this.ws.onerror = (event) => {
      console.error("[signaling] WebSocket error:", event);
    };

    this.ws.onclose = (event) => {
      console.log(`[signaling] WebSocket closed: code=${event.code} reason=${event.reason}`);
    };
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
