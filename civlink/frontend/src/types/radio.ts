export interface RadioState {
  frequency: number;
  mode: string;
  filter: number;
}

export interface ScopeFrame {
  center_hz: number;
  span_hz: number;
  bins: number[];
}
