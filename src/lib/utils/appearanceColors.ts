// Shared by the theme and its contrast regression tests. Inputs are normalised hex colours.
function channels(hex: string): number[] {
  return [1, 3, 5].map(index => parseInt(hex.slice(index, index + 2), 16));
}

function hex(channels: number[]): string {
  return `#${channels.map(channel => Math.round(channel).toString(16).padStart(2, "0")).join("")}`;
}

export function mixColors(color: string, background: string, amount: number): string {
  const back = channels(background);
  return hex(channels(color).map((channel, index) => channel * amount + back[index] * (1 - amount)));
}

function luminance(color: string): number {
  const linear = channels(color).map(value => {
    const channel = value / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
}

export function contrastRatio(first: string, second: string): number {
  const a = luminance(first), b = luminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

function foreground(background: string): string {
  const dark = "#121318", light = "#ffffff";
  const darkContrast = contrastRatio(dark, background), lightContrast = contrastRatio(light, background);
  if (Math.max(darkContrast, lightContrast) < 4.5) return "#000000";
  return darkContrast >= lightContrast ? dark : light;
}

export function accentAppearance(color: string) {
  const hover = mixColors(color, "#000000", 0.92);
  // Accent labels occur on plain surfaces and fills tinted by up to 30%, including hover.
  const backgrounds = ["#121318", "#191b22", "#22252e"]
    .flatMap(background => [background, mixColors(color, background, 0.3)]);
  let text = color;
  for (let step = 0; step <= 100; step++) {
    text = mixColors(color, "#ffffff", 1 - step / 100);
    if (backgrounds.every(background => contrastRatio(text, background) >= 4.5)) break;
  }
  return { color, hover, foreground: foreground(color), hoverForeground: foreground(hover), text, focus: text };
}
