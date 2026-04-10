import type { Component } from "solid-js";

interface VfoDisplayProps {
  frequency: number;
  mode: string;
}

/** Format Hz as grouped digits like RS-BA1: "7.070.00" for 7.070 MHz */
const formatFrequency = (hz: number): { mhz: string; khz: string; hz100: string } => {
  const total = Math.round(hz / 10); // 10 Hz resolution
  const mhzPart = Math.floor(hz / 1_000_000);
  const khzPart = Math.floor((hz % 1_000_000) / 1_000);
  const hzPart = Math.floor((hz % 1_000) / 10);
  return {
    mhz: mhzPart.toString(),
    khz: khzPart.toString().padStart(3, "0"),
    hz100: hzPart.toString().padStart(2, "0"),
  };
};

const VfoDisplay: Component<VfoDisplayProps> = (props) => {
  const freq = () => formatFrequency(props.frequency);
  return (
    <div class="vfo-display">
      <div class="vfo-frequency">
        <span class="freq-mhz">{freq().mhz}</span>
        <span class="freq-dot">.</span>
        <span class="freq-khz">{freq().khz}</span>
        <span class="freq-dot">.</span>
        <span class="freq-hz">{freq().hz100}</span>
      </div>
      <span class="mode-badge">{props.mode}</span>
    </div>
  );
};

export default VfoDisplay;
