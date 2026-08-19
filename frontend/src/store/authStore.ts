import { create } from "zustand";
import { clearRefreshToken, setRefreshToken } from "../utils/secureStorage";
import { useReadReceiptStore } from "./readReceiptStore";
import { useDmPreviewStore } from "./dmPreviewStore";

interface User {
  id: string;
  name: string;
  email: string;
}

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  tokenType?: string;
  expiresIn?: number;
}

interface AuthState {
  user: User | null;
  tokens: TokenPair | null;
  isAuthenticated: boolean;
  isHydrating: boolean;
  isLoading: boolean;
  error: string | null;

  setUser: (user: User) => void;
  setTokens: (tokens: TokenPair) => void;
  login: (tokens: TokenPair, user: User) => void;
  logout: () => void;
  signOut: () => Promise<void>;
  fetchMe: () => Promise<void>;
  setHydrating: (v: boolean) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>()((set, get) => ({
  user: null,
  tokens: null,
  isAuthenticated: false,
  isHydrating: true,
  isLoading: false,
  error: null,

  setUser: (user) => set({ user }),

  setTokens: (tokens) => {
    set({ tokens });
    void setRefreshToken(tokens.refreshToken);
  },

  login: (tokens, user) => {
    set({ tokens, user, isAuthenticated: true, error: null });
    void setRefreshToken(tokens.refreshToken);
  },

  logout: () => {
    set({ user: null, tokens: null, isAuthenticated: false, error: null });
    useReadReceiptStore.getState().clear();
    useDmPreviewStore.getState().clear();
    void import("../utils/avatarCache").then((m) => m.clearAvatarCache());
    void import("./userNamesStore").then((m) =>
      m.useUserNamesStore.getState().clear(),
    );
    void clearRefreshToken();
  },

  signOut: async () => {
    const { authAPI } = await import("../api/auth");
    try {
      await authAPI.logout();
    } catch {}
    get().logout();
  },

  fetchMe: async () => {
    const { userAPI } = await import("../api/user");
    const me = await userAPI.getMe();
    set({ user: { id: me.id, name: me.name, email: me.email } });
    if (me.locale === "ru" || me.locale === "en") {
      const { setLocale, currentLocale } = await import("../i18n");
      if (currentLocale() !== me.locale) {
        setLocale(me.locale);
      }
    }
  },

  setHydrating: (v) => set({ isHydrating: v }),
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
  clearError: () => set({ error: null }),
}));
