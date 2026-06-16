
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

      googleOAuth?: () => Promise<string>;
      googleOAuthAvailable?: () => Promise<boolean>;
    };
  }
}
