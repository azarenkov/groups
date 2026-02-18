// Global type definitions for Electron API exposed via preload script

export {};

declare global {
  interface Window {
    electron: {
      // Window controls
      minimize: () => void;
      maximize: () => void;
      close: () => void;

      // Platform info
      platform: 'darwin' | 'win32' | 'linux' | string;
    };
  }
}
