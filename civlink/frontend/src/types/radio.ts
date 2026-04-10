export interface ScopeFrame {
  center_hz: number;
  span_hz: number;
  bins: number[];
}

export type RadioEvent =
  | { type: "frequency"; data: number }
  | { type: "mode"; data: [string, number] }
  | { type: "scope"; data: ScopeFrame }
  | { type: "signal_meter"; data: number }
  | { type: "rf_power"; data: number }
  | { type: "swr"; data: number }
  | { type: "alc"; data: number };
