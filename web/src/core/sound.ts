// Short procedural tones. On until muted. No audio files.
let ctx: AudioContext | null = null;
let enabled = true;

export const soundOn = (): boolean => enabled;
export function toggleSound(): boolean {
  enabled = !enabled;
  return enabled;
}

function ac(): AudioContext | null {
  if (!enabled) return null;
  try {
    ctx ??= new (window.AudioContext || (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext)();
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

function blip(freq: number, dur = 0.07, gain = 0.05, type: OscillatorType = "triangle"): void {
  const a = ac();
  if (!a) return;
  const o = a.createOscillator();
  const g = a.createGain();
  o.type = type;
  o.frequency.value = freq;
  g.gain.setValueAtTime(gain, a.currentTime);
  g.gain.exponentialRampToValueAtTime(0.0004, a.currentTime + dur);
  o.connect(g).connect(a.destination);
  o.start();
  o.stop(a.currentTime + dur);
}

export const sfx = {
  deal: () => blip(420, 0.06, 0.04),
  place: () => blip(300, 0.08, 0.055),
  draw: () => blip(230, 0.08, 0.05),
  flip: () => blip(520, 0.05, 0.035),
  bad: () => blip(140, 0.12, 0.06, "sawtooth"),
  win: () => {
    blip(392, 0.12, 0.05);
    setTimeout(() => blip(523, 0.12, 0.05), 110);
    setTimeout(() => blip(659, 0.18, 0.05), 220);
  },
};
