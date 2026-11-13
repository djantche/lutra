import { hasBackend } from "./backend";
import { invoke } from "./host";

export interface AudioDevice {
  name: string;
  description: string;
}

/** One direction of the sound: the speakers (`output`) or the microphone (`input`). */
export interface AudioSide {
  devices: AudioDevice[];
  default: string;
  /** 0 to 100. */
  volume: number;
  muted: boolean;
}

export interface AudioStatus {
  output: AudioSide;
  input: AudioSide;
}

export type AudioKind = "output" | "input";

const mock: AudioStatus = {
  output: {
    devices: [
      { name: "speakers", description: "Built-in Audio Analog Stereo" },
      { name: "headphones", description: "Headphones" },
    ],
    default: "speakers",
    volume: 65,
    muted: false,
  },
  input: {
    devices: [{ name: "mic", description: "Built-in Microphone" }],
    default: "mic",
    volume: 72,
    muted: false,
  },
};

/** The system's sound, or null when it cannot be read (no sound server): nothing is invented then. */
export async function audioStatus(): Promise<AudioStatus | null> {
  if (!hasBackend()) return structuredClone(mock);
  try {
    return await invoke<AudioStatus>("audio_status");
  } catch {
    return null;
  }
}

export const setAudioVolume = (kind: AudioKind, percent: number): Promise<void> =>
  hasBackend() ? invoke<void>("set_audio_volume", { kind, percent }) : Promise.resolve();

export const setAudioMuted = (kind: AudioKind, muted: boolean): Promise<void> =>
  hasBackend() ? invoke<void>("set_audio_muted", { kind, muted }) : Promise.resolve();

export const setAudioDevice = (kind: AudioKind, name: string): Promise<void> =>
  hasBackend() ? invoke<void>("set_audio_device", { kind, name }) : Promise.resolve();
