import React from "react";
import { useTranslation } from "react-i18next";
import Icon, { type IconName } from "../../../../shared/ui/Icon";
import { userAPI, type Connection } from "../../../social/api/user";
import { ApiError } from "../../../../shared/lib/apiError";
import LoadingDots from "../../../../shared/ui/LoadingDots";


export function ConnectionsSection() {
  const { t } = useTranslation("settings");
  const [connections, setConnections] = React.useState<Connection[] | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const load = React.useCallback(() => {
    userAPI
      .listConnections()
      .then((items) => {
        setConnections(items);
        setError(null);
      })
      .catch(() => setError(t("connections.loadFailed")));
  }, [t]);
  React.useEffect(() => {
    load();
  }, [load]);
  React.useEffect(() => {
    const onReturn = () => {
      setBusy(false);
      load();
    };
    window.addEventListener("focus", onReturn);
    window.addEventListener("pageshow", onReturn);
    return () => {
      window.removeEventListener("focus", onReturn);
      window.removeEventListener("pageshow", onReturn);
    };
  }, [load]);
  const pollRef = React.useRef<number | null>(null);
  const stopPolling = React.useCallback(() => {
    if (pollRef.current !== null) {
      window.clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);
  const startPolling = React.useCallback(
    (provider: string) => {
      stopPolling();
      let ticks = 0;
      pollRef.current = window.setInterval(() => {
        ticks += 1;
        if (ticks > 48) {
          stopPolling();
          return;
        }
        userAPI
          .listConnections()
          .then((items) => {
            setConnections(items);
            setError(null);
            if (items.some((c) => c.provider === provider)) stopPolling();
          })
          .catch(() => {});
      }, 2500);
    },
    [stopPolling],
  );
  React.useEffect(() => stopPolling, [stopPolling]);
  const providers: { id: string; name: string; icon: IconName; desc: string }[] = [
    { id: "github", name: "GitHub", icon: "github", desc: t("connections.githubDesc") },
    { id: "steam", name: "Steam", icon: "steam", desc: t("connections.steamDesc") },
  ];
  const connect = (provider: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    const popup = window.open("", "_blank");
    userAPI
      .startConnection(provider)
      .then((url) => {
        if (popup) {
          popup.location.href = url;
          startPolling(provider);
        } else {
          window.location.href = url;
        }
      })
      .catch((e) => {
        popup?.close();
        const apiErr = e instanceof ApiError ? e : null;
        setError(
          apiErr?.code === "PROVIDER_NOT_CONFIGURED"
            ? t("connections.notConfigured")
            : t("connections.failed"),
        );
      })
      .finally(() => setBusy(false));
  };
  const disconnect = (provider: string) => {
    if (busy) return;
    setBusy(true);
    setError(null);
    userAPI
      .removeConnection(provider)
      .then(() => load())
      .catch(() => setError(t("connections.failed")))
      .finally(() => setBusy(false));
  };
  return (
    <>
      <h1>{t("connections.title")}</h1>
      <p className="lead">{t("connections.lead")}</p>
      <div className="s-section">
        <h2>{t("connections.available")}</h2>
        {error && <div className="s-error">{error}</div>}
        {!connections && !error ? (
          <div className="s-row"><LoadingDots /></div>
        ) : (
          providers.map((p) => {
            const linked = connections?.find((c) => c.provider === p.id);
            return (
              <div className="conn-row" key={p.id}>
                <div className="conn-ico"><Icon name={p.icon} size={18} /></div>
                <div className="conn-info">
                  <div className="conn-name">{p.name}</div>
                  <div className="conn-meta">
                    {linked
                      ? t("connections.connectedAs", { name: linked.displayName || p.name })
                      : p.desc}
                  </div>
                </div>
                <button
                  className={"conn-action" + (linked ? "" : " connect")}
                  disabled={busy}
                  onClick={() => (linked ? disconnect(p.id) : connect(p.id))}
                >
                  {linked ? t("connections.disconnect") : t("connections.connect")}
                </button>
              </div>
            );
          })
        )}
      </div>
    </>
  );
}
