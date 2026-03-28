let audioCtx: AudioContext | null = null;

function getContext(): AudioContext {
  if (!audioCtx) {
    audioCtx = new AudioContext();
  }
  return audioCtx;
}

type SoundFn = (ctx: AudioContext, gain: GainNode) => void;

const sounds: Record<string, SoundFn> = {
  // Two ascending tones — classic notification chime
  chime(ctx, masterGain) {
    const notes = [523.25, 659.25]; // C5, E5
    notes.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const env = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      const start = ctx.currentTime + i * 0.15;
      env.gain.setValueAtTime(0, start);
      env.gain.linearRampToValueAtTime(0.6, start + 0.02);
      env.gain.exponentialRampToValueAtTime(0.001, start + 0.25);
      osc.connect(env);
      env.connect(masterGain);
      osc.start(start);
      osc.stop(start + 0.25);
    });
  },

  // Single resonant bell tone with harmonic overtone
  bell(ctx, masterGain) {
    const freqs = [440, 880];
    const gains = [0.6, 0.2];
    freqs.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const env = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;
      const t = ctx.currentTime;
      env.gain.setValueAtTime(gains[i], t);
      env.gain.exponentialRampToValueAtTime(0.001, t + 0.6);
      osc.connect(env);
      env.connect(masterGain);
      osc.start(t);
      osc.stop(t + 0.6);
    });
  },

  // Gentle ambient ping — slow attack, soft decay
  soft(ctx, masterGain) {
    const osc = ctx.createOscillator();
    const env = ctx.createGain();
    osc.type = "sine";
    osc.frequency.value = 587.33; // D5
    const t = ctx.currentTime;
    env.gain.setValueAtTime(0, t);
    env.gain.linearRampToValueAtTime(0.4, t + 0.08);
    env.gain.exponentialRampToValueAtTime(0.001, t + 0.5);
    osc.connect(env);
    env.connect(masterGain);
    osc.start(t);
    osc.stop(t + 0.5);
  },

  // Two quick beeps — attention-grabbing alert
  alert(ctx, masterGain) {
    [0, 0.18].forEach((offset) => {
      const osc = ctx.createOscillator();
      const env = ctx.createGain();
      osc.type = "square";
      osc.frequency.value = 880; // A5
      const start = ctx.currentTime + offset;
      env.gain.setValueAtTime(0, start);
      env.gain.linearRampToValueAtTime(0.35, start + 0.01);
      env.gain.setValueAtTime(0.35, start + 0.08);
      env.gain.exponentialRampToValueAtTime(0.001, start + 0.12);
      osc.connect(env);
      env.connect(masterGain);
      osc.start(start);
      osc.stop(start + 0.12);
    });
  },
};

export function playNotificationSound(choice: string, volume: number) {
  try {
    const ctx = getContext();
    const masterGain = ctx.createGain();
    masterGain.gain.value = Math.max(0, Math.min(1, volume / 100));
    masterGain.connect(ctx.destination);

    const play = sounds[choice] || sounds.chime;
    play(ctx, masterGain);
  } catch {
    // Silently fail if audio context unavailable
  }
}

export function previewSound(choice: string, volume: number) {
  playNotificationSound(choice, volume);
}

export function getAvailableSounds(): { value: string; label: string }[] {
  return [
    { value: "chime", label: "Chime" },
    { value: "bell", label: "Bell" },
    { value: "soft", label: "Soft" },
    { value: "alert", label: "Alert" },
  ];
}
