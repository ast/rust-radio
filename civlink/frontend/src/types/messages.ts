export type SignalingMessage =
  | { type: "offer"; payload: RTCSessionDescriptionInit }
  | { type: "answer"; payload: RTCSessionDescriptionInit }
  | { type: "ice-candidate"; payload: RTCIceCandidateInit };
