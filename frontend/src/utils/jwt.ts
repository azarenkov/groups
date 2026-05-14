export interface AccessTokenClaims {
  sub: string;
  exp: number;
  iat: number;
  tokenType: string;
  verified: boolean;
}

function base64UrlDecode(input: string): string {
  const padded = input.replace(/-/g, "+").replace(/_/g, "/");
  const pad = padded.length % 4;
  const final = pad ? padded + "=".repeat(4 - pad) : padded;
  return atob(final);
}

export function decodeAccessToken(token: string): AccessTokenClaims | null {
  const parts = token.split(".");
  if (parts.length !== 3) return null;
  try {
    const payload = JSON.parse(base64UrlDecode(parts[1]));
    return payload as AccessTokenClaims;
  } catch {
    return null;
  }
}

export function isAccessTokenVerified(token: string | undefined | null): boolean {
  if (!token) return false;
  const claims = decodeAccessToken(token);
  return claims?.verified === true;
}
