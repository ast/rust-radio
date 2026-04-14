import { type Component, createSignal, createEffect } from "solid-js";

interface AudioControlsProps {
  audioElement?: HTMLAudioElement;
  rfGain?: number;
  onRfGainChange?: (value: number) => void;
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

  const rfPercent = () => Math.round(((props.rfGain ?? 255) / 255) * 100);
  const setRfPercent = (pct: number) => {
    const clamped = Math.max(0, Math.min(100, pct));
    props.onRfGainChange?.(Math.round((clamped / 100) * 255));
  };

  return (
    <div class="audio-controls-stack">
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
          onWheel={(e) => {
            e.preventDefault();
            const step = e.deltaY < 0 ? 5 : -5;
            setVolume(Math.max(0, Math.min(100, volume() + step)));
          }}
        />
        <span class="vol-value">{volume()}</span>
      </div>
      <div class="audio-controls">
        <span class="ctrl-label">RF</span>
        <input
          type="range"
          min="0"
          max="100"
          value={rfPercent()}
          disabled={props.onRfGainChange === undefined}
          onInput={(e) => setRfPercent(Number(e.currentTarget.value))}
          onWheel={(e) => {
            e.preventDefault();
            const step = e.deltaY < 0 ? 5 : -5;
            setRfPercent(rfPercent() + step);
          }}
        />
        <span class="vol-value">{rfPercent()}</span>
      </div>
    </div>
  );
};

export default AudioControls;
