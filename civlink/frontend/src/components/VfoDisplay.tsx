import type { Component } from "solid-js";

interface VfoDisplayProps {
  frequency: number;
  mode: string;
}

const formatFrequency = (hz: number): string => {
  const mhz = hz / 1_000_000;
  return mhz.toFixed(6);
};

const VfoDisplay: Component<VfoDisplayProps> = (props) => {
  return (
    <div class="vfo-display">
      <span class="frequency">{formatFrequency(props.frequency)} MHz</span>
      <span class="mode">{props.mode}</span>
    </div>
  );
};

export default VfoDisplay;
