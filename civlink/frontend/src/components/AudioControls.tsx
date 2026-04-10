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
      <span class="ctrl-label">AF</span>
      <button
        class={muted() ? "muted" : ""}
        onClick={() => setMuted(!muted())}
      >
        {muted() ? "Muted" : "Mute"}
      </button>
      <input
        type="range"
        min="0"
        max="100"
        value={volume()}
        onInput={(e) => setVolume(Number(e.currentTarget.value))}
      />
      <span class="vol-value">{volume()}</span>
    </div>
  );
};

export default AudioControls;
