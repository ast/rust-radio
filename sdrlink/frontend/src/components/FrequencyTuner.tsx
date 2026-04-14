import { Component } from "solid-js";

interface Props {
  hz: number;
  onTune: (hz: number) => void;
}

const format = (hz: number) => {
  const ghz = Math.floor(hz / 1_000_000_000);
  const mhz = Math.floor((hz % 1_000_000_000) / 1_000_000);
  const khz = Math.floor((hz % 1_000_000) / 1_000);
  const hz10 = Math.floor((hz % 1_000) / 10);
  return {
    ghz: ghz.toString(),
    mhz: mhz.toString().padStart(3, "0"),
    khz: khz.toString().padStart(3, "0"),
    hz10: hz10.toString().padStart(2, "0"),
  };
};

const FrequencyTuner: Component<Props> = (props) => {
  const f = () => format(props.hz);

  const wheel = (step: number) => (e: WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY < 0 ? step : -step;
    props.onTune(Math.max(0, props.hz + delta));
  };

  return (
    <div class="freq-display">
      <span class="freq-group" onWheel={wheel(1_000_000_000)}>{f().ghz}</span>
      <span class="freq-dot">.</span>
      <span class="freq-group" onWheel={wheel(1_000_000)}>{f().mhz}</span>
      <span class="freq-dot">.</span>
      <span class="freq-group" onWheel={wheel(1_000)}>{f().khz}</span>
      <span class="freq-dot">.</span>
      <span class="freq-group" onWheel={wheel(10)}>{f().hz10}</span>
      <span class="freq-unit">GHz</span>
    </div>
  );
};

export default FrequencyTuner;
