import { http } from "../../../shared/api/http";
import { useAuthStore } from "../store/authStore";

export interface TokenPair {
  accessToken: string;
  refreshToken: string;
  tokenType: string;
  expiresIn: number;
  requiresVerification?: boolean;
}

export type GoogleAuthResult =
  | ({ needsUsername: false } & TokenPair)
  | { needsUsername: true };

class AuthAPI {
  private client = http;
  private refreshInProgress: boolean = false;
  private refreshPromise: Promise<void> | null = null;

  async register(
    name: string,
    email: string,
    password: string,
    locale?: string,
  ): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/register", {
      name,
      email,
      password,
      ...(locale ? { locale } : {}),
    });
    return response.data.data;
  }

  async googleLogin(
    idToken: string,
    locale?: string,
  ): Promise<GoogleAuthResult> {
    const response = await this.client.post("/v1/auth/google", {
      idToken,
      ...(locale ? { locale } : {}),
    });
    return response.data.data;
  }

  async googleComplete(
    idToken: string,
    username: string,
    locale?: string,
  ): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/google/complete", {
      idToken,
      username,
      ...(locale ? { locale } : {}),
    });
    return response.data.data;
  }

  async updateLocale(locale: string): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    await this.client.patch(
      "/v1/users/me/locale",
      { locale },
      {
        headers: tokens?.accessToken
          ? { Authorization: `Bearer ${tokens.accessToken}` }
          : undefined,
      },
    );
  }

  async requestVerificationCode(
    channel: "email" | "phone" = "email",
  ): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    await this.client.post(
      "/v1/auth/request_verification_code",
      { channel },
      {
        headers: tokens?.accessToken
          ? { Authorization: `Bearer ${tokens.accessToken}` }
          : undefined,
      },
    );
  }

  async confirmRegistration(verificationCode: string): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    await this.client.post(
      "/v1/auth/confirm_registration",
      { verificationCode },
      {
        headers: tokens?.accessToken
          ? { Authorization: `Bearer ${tokens.accessToken}` }
          : undefined,
      },
    );
  }

  async login(email: string, password: string): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/login", {
      email,
      password,
    });
    return response.data.data;
  }

  async refresh(): Promise<TokenPair> {
    const tokens = useAuthStore.getState().tokens;
    if (!tokens?.refreshToken) {
      throw new Error("No refresh token available");
    }
    return this.refreshWith(tokens.refreshToken);
  }

  async refreshWith(refreshToken: string): Promise<TokenPair> {
    const response = await this.client.post("/v1/auth/refresh", {
      refreshToken,
    });
    return response.data.data;
  }

  async logout(): Promise<void> {
    const tokens = useAuthStore.getState().tokens;
    if (!tokens?.refreshToken || !tokens?.accessToken) {
      return;
    }

    await this.client.post(
      "/v1/auth/logout",
      { refreshToken: tokens.refreshToken },
      { headers: { Authorization: `Bearer ${tokens.accessToken}` } },
    );
  }

  public async handleRefreshFlow(
    opts: { logoutOnFailure?: boolean } = {},
  ): Promise<void> {
    const logoutOnFailure = opts.logoutOnFailure ?? true;

    if (this.refreshInProgress) {
      if (!this.refreshPromise) return;
      return this.refreshPromise;
    }

    this.refreshInProgress = true;
    this.refreshPromise = (async () => {
      try {
        const newTokens = await this.refresh();

        useAuthStore.getState().setTokens(newTokens);
      } catch (err) {
        if (logoutOnFailure) {
          useAuthStore.getState().logout();
        }
        throw err;
      } finally {
        this.refreshInProgress = false;
        this.refreshPromise = null;
      }
    })();

    return this.refreshPromise;
  }

}

export const authAPI = new AuthAPI();
