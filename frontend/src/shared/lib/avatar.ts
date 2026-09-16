export const AVATAR_PALETTE = [
  "#0071e3",
  "#30a46c",
  "#ff9f0a",
  "#bf5af2",
  "#ff6b35",
  "#5b6cd9",
  "#d97557",
  "#30d158",
];

const hash = (s: string): number => {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h;
};

export const colorForId = (id: string): string => {
  if (!id) return AVATAR_PALETTE[0];
  return AVATAR_PALETTE[hash(id) % AVATAR_PALETTE.length];
};

export const initialFor = (name: string): string => {
  const t = name.trim();
  return t ? t.charAt(0).toUpperCase() : "?";
};

export const glyphFor = (name: string): string => {
  const t = name.trim();
  if (!t) return "?";
  return t
    .split(/\s+/)
    .slice(0, 2)
    .map((w) => w[0])
    .join("")
    .toUpperCase();
};
