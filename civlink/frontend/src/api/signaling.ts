import type { SignalingMessage } from "../types/messages";

export class SignalingClient {
  private ws: WebSocket | null = null;
  private onMessage: ((msg: SignalingMessage) => void) | null = null;

  connect(token: string): void {
    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    this.ws = new WebSocket(`${protocol}//${window.location.host}/api/ws?token=${token}`);

    this.ws.onmessage = (event) => {
      const msg: SignalingMessage = JSON.parse(event.data);
      this.onMessage?.(msg);
    };
  }

  send(msg: SignalingMessage): void {
    this.ws?.send(JSON.stringify(msg));
  }

  setOnMessage(handler: (msg: SignalingMessage) => void): void {
    this.onMessage = handler;
  }

  close(): void {
    this.ws?.close();
    this.ws = null;
  }
}
