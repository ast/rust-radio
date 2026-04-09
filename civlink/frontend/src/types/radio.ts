export interface RadioState {
  frequency: number;
  mode: string;
  ptt: boolean;
}

export interface ScopeFrame {
  center_hz: number;
  span_hz: number;
  bins: number[];
}
