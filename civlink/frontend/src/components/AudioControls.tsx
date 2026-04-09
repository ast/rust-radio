import { type Component, createSignal } from "solid-js";

const AudioControls: Component = () => {
  const [volume, setVolume] = createSignal(80);
  const [muted, setMuted] = createSignal(false);

  return (
    <div class="audio-controls">
      <button onClick={() => setMuted(!muted())}>
        {muted() ? "Unmute" : "Mute"}
      </button>
      <input
        type="range"
        min="0"
        max="100"
        value={volume()}
        onInput={(e) => setVolume(Number(e.currentTarget.value))}
      />
      <span>{volume()}%</span>
    </div>
  );
};

export default AudioControls;
