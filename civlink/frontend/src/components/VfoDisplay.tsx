import { type Component, For } from "solid-js";

interface VfoDisplayProps {
  frequency: number;
  mode: string;
  onTune?: (hz: number) => void;
  onModeChange?: (mode: string) => void;
}

const MODES = ["Lsb", "Usb", "Am", "Cw", "Fm", "Rtty", "CwR", "RttyR", "DataLsb", "DataUsb", "DataFm"] as const;

const MODE_LABELS: Record<string, string> = {
  Lsb: "LSB", Usb: "USB", Am: "AM", Cw: "CW", Fm: "FM",
  Rtty: "RTTY", CwR: "CW-R", RttyR: "RTTY-R",
  DataLsb: "D-LSB", DataUsb: "D-USB", DataFm: "D-FM",
};

/** Format Hz as grouped digits like RS-BA1: "7.070.00" for 7.070 MHz */
const formatFrequency = (hz: number): { mhz: string; khz: string; hz100: string } => {
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

  const handleWheel = (step: number) => (e: WheelEvent) => {
    e.preventDefault();
    if (!props.onTune) return;
    const delta = e.deltaY < 0 ? step : -step;
    props.onTune(props.frequency + delta);
  };

  return (
    <div class="vfo-display">
      <div class="vfo-frequency">
        <span class="freq-mhz freq-tunable" onWheel={handleWheel(1_000_000)}>{freq().mhz}</span>
        <span class="freq-dot">.</span>
        <span class="freq-khz freq-tunable" onWheel={handleWheel(1_000)}>{freq().khz}</span>
        <span class="freq-dot">.</span>
        <span class="freq-hz freq-tunable" onWheel={handleWheel(100)}>{freq().hz100}</span>
      </div>
      <div class="mode-selector">
        <For each={[...MODES]}>{(m) =>
          <button
            class={`mode-btn ${props.mode === m ? "mode-btn-active" : ""}`}
            onClick={() => props.onModeChange?.(m)}
          >
            {MODE_LABELS[m]}
          </button>
        }</For>
      </div>
    </div>
  );
};

export default VfoDisplay;
