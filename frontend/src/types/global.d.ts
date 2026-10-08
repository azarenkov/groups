
export {};

declare global {
  interface Window {
    electron?: {
      minimize: () => void;
      maximize: () => void;
      close: () => void;

      platform: 'darwin' | 'win32' | 'linux' | string;

      secureGet: () => Promise<string | null>;
      secureSet: (value: string) => Promise<boolean>;
      secureClear: () => Promise<boolean>;

      onScreenShareSources?: (
        callback: (data: {
          requestId: number;
          sources: {
            id: string;
            name: string;
            kind: "screen" | "window";
            thumbnail: string;
          }[];
        }) => void,
      ) => () => void;
      onDeepLink?: (callback: (url: string) => void) => () => void;
      getPendingDeepLink?: () => Promise<string | null>;
      reportRendered?: () => void;
      setGlobalPushToTalk?: (code: string | null) => Promise<boolean>;
      onPushToTalk?: (
        callback: (state: PushToTalkKeyState) => void,
      ) => () => void;
      isFullscreen?: () => Promise<boolean>;
      onFullscreenChange?: (callback: (value: boolean) => void) => () => void;
      pickScreenShareSource?: (
        requestId: number,
        sourceId: string | null,
      ) => void;
      getGameActivity?: () => Promise<GameActivityInfo | null>;
      onGameActivity?: (
        callback: (activity: GameActivityInfo | null) => void,
      ) => () => void;
    };
  }
}

export interface PushToTalkKeyState {
  down: boolean;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

export interface GameActivityInfo {
  gameId: string;
  name: string;
  startedAt: string;
}

export interface ScreenShareSource {
  id: string;
  name: string;
  kind: "screen" | "window";
  thumbnail: string;
}
