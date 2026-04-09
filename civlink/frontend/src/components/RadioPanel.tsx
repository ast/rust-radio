import type { Component } from "solid-js";
import VfoDisplay from "./VfoDisplay";
import Spectrum from "./Spectrum";
import AudioControls from "./AudioControls";

interface RadioPanelProps {
  token: string;
}

const RadioPanel: Component<RadioPanelProps> = (props) => {
  // TODO: establish WebRTC connection using props.token

  return (
    <div class="radio-panel">
      <VfoDisplay frequency={14074000} mode="USB" />
      <Spectrum bins={[]} />
      <AudioControls />
    </div>
  );
};

export default RadioPanel;
