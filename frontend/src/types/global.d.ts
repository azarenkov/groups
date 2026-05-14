
export {};

declare global {
  interface Window {
    electron: {
      minimize: () => void;
      maximize: () => void;
      close: () => void;

      platform: 'darwin' | 'win32' | 'linux' | string;
    };
  }
}
