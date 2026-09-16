import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { currentLocale, setLocale } from "../../../../i18n";


export function LanguageSection() {
  const { t, i18n } = useTranslation("settings");
  void i18n.language;
  const active = currentLocale();
  const langs: { v: "en" | "ru"; n: string }[] = [
    { v: "en", n: "English" },
    { v: "ru", n: "Русский" },
  ];

  const apply = (next: "en" | "ru") => {
    setLocale(next);
    void (async () => {
      try {
        const { authAPI } = await import("../../../auth/api/auth");
        await authAPI.updateLocale(next);
      } catch (err) {
        console.error("[Settings] updateLocale failed:", err);
      }
    })();
  };

  return (
    <>
      <h1>{t("language.title")}</h1>
      <p className="lead">{t("language.lead")}</p>
      <div className="s-section">
        <h2>{t("language.displayLanguage")}</h2>
        {langs.map((l) => (
          <button
            key={l.v}
            className="s-row"
            onClick={() => apply(l.v)}
            style={{
              width: "100%",
              border: 0,
              background: "transparent",
              cursor: "pointer",
              textAlign: "left",
            }}
          >
            <div className="s-row-text">
              <div className="lbl">{l.n}</div>
              <div className="hint">{l.v.toUpperCase()}</div>
            </div>
            <div className="s-row-control">
              {active === l.v && (
                <Icon name="check" size={18} style={{ color: "var(--accent)" }} />
              )}
            </div>
          </button>
        ))}
      </div>
    </>
  );
}
