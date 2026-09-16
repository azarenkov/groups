import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { GoogleGlyph } from "./authHelpers";

const GIS_SRC = "https://accounts.google.com/gsi/client";

interface GoogleCredentialResponse {
  credential?: string;
}

interface Props {
  onToken: (idToken: string) => void;
  text?: "signin_with" | "signup_with" | "continue_with";
}

let scriptPromise: Promise<void> | null = null;

function loadGis(): Promise<void> {
  if (typeof window === "undefined") return Promise.reject(new Error("no window"));
  const w = window as unknown as { google?: unknown };
  if (w.google) return Promise.resolve();
  if (scriptPromise) return scriptPromise;
  scriptPromise = new Promise<void>((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>(`script[src="${GIS_SRC}"]`);
    if (existing) {
      existing.addEventListener("load", () => resolve());
      existing.addEventListener("error", () => reject(new Error("GIS failed to load")));
      return;
    }
    const script = document.createElement("script");
    script.src = GIS_SRC;
    script.async = true;
    script.defer = true;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("GIS failed to load"));
    document.head.appendChild(script);
  });
  return scriptPromise;
}

const CLIENT_ID = process.env.REACT_APP_GOOGLE_CLIENT_ID ?? "";

function originSupportsGis(): boolean {
  if (typeof window === "undefined") return false;
  return /^https?:$/.test(window.location.protocol);
}

function desktopFlow(): (() => Promise<string>) | undefined {
  if (typeof window === "undefined") return undefined;
  return window.electron?.googleOAuth;
}

const WEB_GIS_READY = !desktopFlow() && !!CLIENT_ID && originSupportsGis();

export const GoogleSignInButton: React.FC<Props> = ({ onToken, text = "continue_with" }) => {
  const { t } = useTranslation("auth");
  const containerRef = useRef<HTMLDivElement>(null);
  const onTokenRef = useRef(onToken);
  onTokenRef.current = onToken;

  const desktop = desktopFlow();
  const [busy, setBusy] = useState(false);
  const [desktopReady, setDesktopReady] = useState(false);

  useEffect(() => {
    if (!desktop) return;
    let cancelled = false;
    const check = window.electron?.googleOAuthAvailable;
    if (!check) return;
    check()
      .then((ok) => {
        if (!cancelled) setDesktopReady(!!ok);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [desktop]);

  useEffect(() => {
    if (!WEB_GIS_READY) return;
    let cancelled = false;

    loadGis()
      .then(() => {
        if (cancelled || !containerRef.current) return;
        const google = (window as unknown as { google: any }).google;
        if (!google?.accounts?.id) return;

        google.accounts.id.initialize({
          client_id: CLIENT_ID,
          callback: (resp: GoogleCredentialResponse) => {
            if (resp.credential) onTokenRef.current(resp.credential);
          },
        });

        containerRef.current.innerHTML = "";
        google.accounts.id.renderButton(containerRef.current, {
          type: "standard",
          theme: "filled_black",
          size: "large",
          text,
          shape: "rectangular",
          width: containerRef.current.offsetWidth || 320,
          logo_alignment: "center",
        });
      })
      .catch((err) => {
        console.warn("[GoogleSignInButton] init failed:", err);
      });

    return () => {
      cancelled = true;
    };
  }, [text]);

  if (desktop) {
    if (!desktopReady) return null;
    const onClick = async () => {
      if (busy) return;
      setBusy(true);
      try {
        const idToken = await desktop();
        if (idToken) onTokenRef.current(idToken);
      } catch (err) {
        console.warn("[GoogleSignInButton] desktop oauth failed:", err);
      } finally {
        setBusy(false);
      }
    };
    return (
      <>
        <div className="auth-divider">{t("orContinueWith")}</div>
        <div className="auth-social">
          <button
            type="button"
            className="btn social"
            onClick={onClick}
            disabled={busy}
          >
            <GoogleGlyph />
            {t("continueWithGoogle")}
          </button>
        </div>
      </>
    );
  }

  if (!WEB_GIS_READY) return null;

  return (
    <>
      <div className="auth-divider">{t("orContinueWith")}</div>
      <div className="auth-social">
        <div className="google-gis-wrap">
          <button type="button" className="btn social" tabIndex={-1} aria-hidden="true">
            <GoogleGlyph />
            {t("continueWithGoogle")}
          </button>
          <div ref={containerRef} className="google-gis-overlay" />
        </div>
      </div>
    </>
  );
};

export default GoogleSignInButton;
