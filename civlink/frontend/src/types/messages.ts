import type { RadioCommand, RadioEvent } from "./radio";

export type SignalingMessage =
  | { type: "offer"; payload: RTCSessionDescriptionInit }
  | { type: "answer"; payload: RTCSessionDescriptionInit }
  | { type: "ice-candidate"; payload: RTCIceCandidateInit }
  | { type: "radio-event"; payload: RadioEvent }
  | { type: "radio-command"; payload: RadioCommand };
