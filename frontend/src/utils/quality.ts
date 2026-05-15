import { ConnectionQuality } from "../services/livekit";

export const qualityLabel = (q: ConnectionQuality): string => {
  switch (q) {
    case ConnectionQuality.Excellent:
      return "Excellent";
    case ConnectionQuality.Good:
      return "Good";
    case ConnectionQuality.Poor:
      return "Poor";
    case ConnectionQuality.Lost:
      return "Lost";
    default:
      return "Unknown";
  }
};

export const qualityClass = (q: ConnectionQuality): string => {
  switch (q) {
    case ConnectionQuality.Excellent:
      return "excellent";
    case ConnectionQuality.Good:
      return "good";
    case ConnectionQuality.Poor:
      return "poor";
    case ConnectionQuality.Lost:
      return "lost";
    default:
      return "unknown";
  }
};

export const qualityActiveBars = (q: ConnectionQuality): number => {
  switch (q) {
    case ConnectionQuality.Excellent:
      return 3;
    case ConnectionQuality.Good:
      return 2;
    case ConnectionQuality.Poor:
      return 1;
    case ConnectionQuality.Lost:
      return 0;
    default:
      return 0;
  }
};
