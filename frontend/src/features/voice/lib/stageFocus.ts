export const AUTO_HOLD_MS = 700;
export const AUTO_MIN_GAP_MS = 1500;

export interface StageCandidate {
  id: string;
  speaking: boolean;
  hasVideo: boolean;
  isSelf: boolean;
}

export interface AutoFocus {
  current: string | null;
  candidate: string | null;
  candidateSince: number;
  switchedAt: number;
}

export const IDLE_AUTO_FOCUS: AutoFocus = {
  current: null,
  candidate: null,
  candidateSince: 0,
  switchedAt: Number.NEGATIVE_INFINITY,
};

const videoPool = (entries: StageCandidate[]): StageCandidate[] => {
  const withVideo = entries.filter((e) => e.hasVideo);
  const others = withVideo.filter((e) => !e.isSelf);
  return others.length > 0 ? others : withVideo;
};

export function nextAutoFocus(state: AutoFocus, entries: StageCandidate[], now: number): AutoFocus {
  const pool = videoPool(entries);
  const current = pool.find((e) => e.id === state.current);
  if (!current) {
    const pick = pool.find((e) => e.speaking) ?? pool[0] ?? null;
    if (!pick) return IDLE_AUTO_FOCUS;
    return { current: pick.id, candidate: null, candidateSince: 0, switchedAt: now };
  }
  const speaker = current.speaking ? undefined : pool.find((e) => e.speaking && e.id !== current.id);
  if (!speaker) {
    return state.candidate === null ? state : { ...state, candidate: null, candidateSince: 0 };
  }
  const since = state.candidate === speaker.id ? state.candidateSince : now;
  if (now - since >= AUTO_HOLD_MS && now - state.switchedAt >= AUTO_MIN_GAP_MS) {
    return { current: speaker.id, candidate: null, candidateSince: 0, switchedAt: now };
  }
  if (state.candidate === speaker.id) return state;
  return { ...state, candidate: speaker.id, candidateSince: since };
}

export function autoFocusWait(state: AutoFocus, now: number): number | null {
  if (state.candidate === null) return null;
  const hold = AUTO_HOLD_MS - (now - state.candidateSince);
  const gap = AUTO_MIN_GAP_MS - (now - state.switchedAt);
  return Math.max(0, hold, gap);
}
