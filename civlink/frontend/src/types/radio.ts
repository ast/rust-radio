export type ScopeFreq =
  | { mode: "center"; center_hz: number; span_hz: number }
  | { mode: "fixed"; lower_hz: number; upper_hz: number };

export interface ScopeFrame {
  freq: ScopeFreq;
  bins: number[];
}

export type RadioEvent =
  | { type: "frequency"; data: number }
  | { type: "mode"; data: [string, number] }
  | { type: "scope"; data: ScopeFrame }
  | { type: "signal_meter"; data: number }
  | { type: "rf_power"; data: number }
  | { type: "swr"; data: number }
  | { type: "alc"; data: number }
  | { type: "rf_gain"; data: number };

export type ScopeMode = "center" | "fixed";

export type RadioCommand =
  | { type: "set_frequency"; data: number }
  | { type: "set_mode"; data: string }
  | { type: "set_scope_span"; data: number }
  | { type: "set_scope_mode"; data: ScopeMode }
  | { type: "set_scope_fixed_edge"; data: number }
  | { type: "set_rf_gain"; data: number };
