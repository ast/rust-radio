import { type Component, For } from "solid-js";

interface VfoDisplayProps {
  frequency: number;
  mode: string;
  onTune?: (hz: number) => void;
  onModeChange?: (mode: string) => void;
  onBandSelect?: (hz: number, mode: string) => void;
}

const MODES = ["Lsb", "Usb", "Am", "Cw", "Fm", "Rtty", "CwR", "RttyR", "DataLsb", "DataUsb", "DataFm"] as const;

const MODE_LABELS: Record<string, string> = {
  Lsb: "LSB", Usb: "USB", Am: "AM", Cw: "CW", Fm: "FM",
  Rtty: "RTTY", CwR: "CW-R", RttyR: "RTTY-R",
  DataLsb: "D-LSB", DataUsb: "D-USB", DataFm: "D-FM",
};

// Common activity frequencies per band with typical mode
const BANDS: { label: string; hz: number; mode: string }[] = [
  { label: "1.8",  hz:  1_840_000, mode: "Cw"  },
  { label: "3.5",  hz:  3_573_000, mode: "Usb" },
  { label: "7",    hz:  7_074_000, mode: "Usb" },
  { label: "10",   hz: 10_136_000, mode: "Usb" },
  { label: "14",   hz: 14_074_000, mode: "Usb" },
  { label: "18",   hz: 18_100_000, mode: "Usb" },
  { label: "21",   hz: 21_074_000, mode: "Usb" },
  { label: "24",   hz: 24_915_000, mode: "Usb" },
  { label: "28",   hz: 28_074_000, mode: "Usb" },
  { label: "50",   hz: 50_313_000, mode: "Usb" },
  { label: "144",  hz: 144_174_000, mode: "Usb" },
  { label: "430",  hz: 432_174_000, mode: "Usb" },
];

/** Format Hz as grouped digits: MHz.KHz.Hz */
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

/** Determine which band label the current frequency falls in */
const activeBand = (hz: number): string | null => {
  const ranges: [string, number, number][] = [
    ["1.8",  1_800_000,   1_999_999],
    ["3.5",  3_400_000,   4_099_999],
    ["7",    6_900_000,   7_499_999],
    ["10",   9_900_000,  10_499_999],
    ["14",  13_900_000,  14_499_999],
    ["18",  17_900_000,  18_499_999],
    ["21",  20_900_000,  21_499_999],
    ["24",  24_400_000,  25_099_999],
    ["28",  28_000_000,  29_999_999],
    ["50",  50_000_000,  54_000_000],
    ["144", 144_000_000, 148_000_000],
    ["430", 420_000_000, 450_000_000],
  ];
  for (const [label, lo, hi] of ranges) {
    if (hz >= lo && hz <= hi) return label;
  }
  return null;
};

interface DigitProps {
  value: string;
  step: number;
  colorClass: string;
  tune: (delta: number) => void;
}

const Digit: Component<DigitProps> = (p) => {
  const onWheel = (e: WheelEvent) => {
    e.preventDefault();
    p.tune(e.deltaY < 0 ? p.step : -p.step);
  };
  return (
    <span class="freq-group">
      <button class="freq-step-btn" onClick={() => p.tune(p.step)} aria-label="increment">▲</button>
      <span class={`${p.colorClass} freq-tunable`} onWheel={onWheel}>{p.value}</span>
      <button class="freq-step-btn" onClick={() => p.tune(-p.step)} aria-label="decrement">▼</button>
    </span>
  );
};

const VfoDisplay: Component<VfoDisplayProps> = (props) => {
  const freq = () => formatFrequency(props.frequency);
  const currentBand = () => activeBand(props.frequency);

  const tune = (delta: number) => {
    if (!props.onTune || props.frequency <= 0) return;
    props.onTune(props.frequency + delta);
  };

  return (
    <>
      <div class="vfo-section">
        <div class="vfo-row">
          <div class="vfo-frequency">
            <Digit value={freq().mhz}   step={1_000_000} colorClass="freq-mhz" tune={tune} />
            <span class="freq-dot">.</span>
            <Digit value={freq().khz}   step={1_000}     colorClass="freq-khz" tune={tune} />
            <span class="freq-dot">.</span>
            <Digit value={freq().hz100} step={100}       colorClass="freq-hz"  tune={tune} />
            <span class="freq-unit">MHz</span>
          </div>
          <div class="mode-panel">
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
      </div>
      <div class="band-section">
        <span class="band-label">Band</span>
        <div class="band-buttons">
          <For each={BANDS}>{(band) =>
            <button
              class={`band-btn ${currentBand() === band.label ? "band-btn-active" : ""}`}
              onClick={() => props.onBandSelect?.(band.hz, band.mode)}
            >
              {band.label}
            </button>
          }</For>
        </div>
      </div>
    </>
  );
};

export default VfoDisplay;
