import { create } from "zustand";
import type { UserRole } from "../../../shared/types/userRole";
import { clearPersistentSessionData } from "../../../shared/lib/persistentSessionData";
import { clearRefreshToken, setRefreshToken } from "../../../shared/lib/secureStorage";
import { resetSessionStores } from "../../../shared/store/resetSession";

interface User {
  id: string;
  name: string;
  email: string;
  emailVerified?: boolean;
  passwordChangedAt?: string | null;
  bio?: string;
  showActivity?: boolean;
  timezone?: string;
  appearOffline?: boolean;
  role?: UserRole;
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
    resetSessionStores();
    clearPersistentSessionData();
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
    const { userAPI } = await import("../../social/api/user");
    const me = await userAPI.getMe();
    const { useUserProfileStore, profileVersion } = await import("../../social/store/userProfileStore");
    const version = profileVersion(me.profileUpdatedAt);
    const cached = useUserProfileStore.getState().bios[me.id];
    const stale = !!cached && cached.at > version;
    if (!stale) useUserProfileStore.getState().apply(me.id, me.bio, version);
    if (me.emojiStatus !== undefined) {
      const { useEmojiStatusStore } = await import("../../social/store/emojiStatusStore");
      useEmojiStatusStore.getState().apply(me.id, me.emojiStatus, version);
    }
    const current = get().user;
    set({
      user: {
        id: me.id,
        name: me.name,
        email: me.email,
        emailVerified: me.emailVerified,
        passwordChangedAt: me.passwordChangedAt,
        bio: stale && current?.id === me.id ? current.bio : me.bio,
        showActivity: stale && current?.id === me.id ? current.showActivity : me.showActivity,
        timezone: me.timezone,
        appearOffline: me.appearOffline,
        role: me.role,
      },
    });
    const localZone = (() => {
      try {
        return Intl.DateTimeFormat().resolvedOptions().timeZone ?? "";
      } catch {
        return "";
      }
    })();
    if (localZone && localZone !== me.timezone) {
      userAPI
        .updateProfile({ timezone: localZone })
        .then((saved) => {
          const u = get().user;
          if (u && u.id === me.id) set({ user: { ...u, timezone: localZone, bio: saved.bio, showActivity: saved.showActivity } });
        })
        .catch((err) => console.warn("[auth] timezone sync failed:", err));
    }
    if (me.locale === "ru" || me.locale === "en") {
      const { setLocale, currentLocale } = await import("../../../i18n");
      if (currentLocale() !== me.locale) {
        setLocale(me.locale);
      }
    }
    const { useAppearanceStore } = await import("../../settings/store/appearanceStore");
    void useAppearanceStore.getState().syncFromServer(me.id);
    const { useVoiceSettingsStore } = await import("../../voice/store/voiceSettingsStore");
    void useVoiceSettingsStore.getState().syncFromServer(me.id);
    const { useNotificationSettingsStore } = await import(
      "../../settings/store/notificationSettingsStore"
    );
    void useNotificationSettingsStore.getState().syncFromServer(me.id);
    const { useHotkeySettingsStore } = await import("../../settings/store/hotkeySettingsStore");
    void useHotkeySettingsStore.getState().syncFromServer(me.id);
  },

  setHydrating: (v) => set({ isHydrating: v }),
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
  clearError: () => set({ error: null }),
}));
