import { type Component, onMount } from "solid-js";

interface SpectrumProps {
  bins: number[];
}

const Spectrum: Component<SpectrumProps> = (props) => {
  let canvas: HTMLCanvasElement | undefined;

  onMount(() => {
    // TODO: draw spectrum bins on canvas
  });

  return (
    <div class="spectrum">
      <canvas ref={canvas} width={800} height={200} />
    </div>
  );
};

export default Spectrum;
