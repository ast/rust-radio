import { type Component, createSignal, createEffect } from "solid-js";

interface AudioControlsProps {
  audioElement?: HTMLAudioElement;
}

const AudioControls: Component<AudioControlsProps> = (props) => {
  const [volume, setVolume] = createSignal(80);
  const [muted, setMuted] = createSignal(false);

  createEffect(() => {
    if (props.audioElement) {
      props.audioElement.muted = muted();
      props.audioElement.volume = volume() / 100;
    }
  });

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
