import React from "react";
import { useTranslation } from "react-i18next";
import Icon from "../../../../shared/ui/Icon";
import { useVoiceSettingsStore } from "../../../voice/store/voiceSettingsStore";
import { useHotkeySettingsStore } from "../../store/hotkeySettingsStore";
import { bindingBlocker, pushToTalkReserves, defaultClaimedBy, effectiveBinding, useActiveBindings } from "../../lib/hotkeys";
import { DESKTOP_SHORTCUTS, HOTKEY_ACTIONS, bindingFromEvent, bindingKeycaps, bindingVerdict, codeLabel, desktopReservedId, hotkeyAction, modifierGlyphs, type HotkeyActionId, type HotkeyGroup } from "../../lib/hotkeyMatching";
import type { SectionId } from "../types";


interface ShortcutsSectionProps {
  onNavigate: (id: SectionId) => void;
}

const SHORTCUT_GROUPS: HotkeyGroup[] = ["navigation", "messaging", "voice"];

export function ShortcutsSection({ onNavigate }: ShortcutsSectionProps) {
  const { t } = useTranslation("settings");
  const pttKey = useVoiceSettingsStore((s) => s.pttKey);
  const pushToTalk = useVoiceSettingsStore((s) => s.pushToTalk);
  const bindings = useActiveBindings();
  const setBinding = useHotkeySettingsStore((s) => s.setBinding);
  const resetAll = useHotkeySettingsStore((s) => s.resetAll);
  const [capturing, setCapturing] = React.useState<HotkeyActionId | null>(null);
  const [notice, setNotice] = React.useState<{ id: HotkeyActionId; text: string } | null>(null);
  const noticeTimer = React.useRef<number | null>(null);

  const currentBinding = React.useCallback(
    (id: HotkeyActionId): string | null => effectiveBinding(bindings, id),
    [bindings],
  );
  const actionName = React.useCallback(
    (id: string): string => t(`shortcuts.actions.${id}`),
    [t],
  );
  const showNotice = React.useCallback((id: HotkeyActionId, text: string) => {
    setNotice({ id, text });
    if (noticeTimer.current !== null) window.clearTimeout(noticeTimer.current);
    noticeTimer.current = window.setTimeout(() => {
      noticeTimer.current = null;
      setNotice(null);
    }, 2500);
  }, []);
  React.useEffect(
    () => () => {
      if (noticeTimer.current !== null) window.clearTimeout(noticeTimer.current);
    },
    [],
  );

  React.useEffect(() => {
    if (!capturing) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (/^(Shift|Control|Alt|Meta)(Left|Right)$/.test(e.code)) return;
      if (e.code === "Escape" && !e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
        setCapturing(null);
        return;
      }
      const binding = bindingFromEvent(e);
      if (!binding) {
        showNotice(capturing, t("shortcuts.unsupportedKey"));
        return;
      }
      const action = hotkeyAction(capturing);
      if (bindingVerdict(action, binding) !== "ok") {
        showNotice(capturing, t("shortcuts.needModifier", { mods: modifierGlyphs() }));
        return;
      }
      const clash =
        HOTKEY_ACTIONS.find((a) => a.id !== capturing && currentBinding(a.id) === binding) ??
        (window.electron && desktopReservedId(binding)
          ? { id: desktopReservedId(binding) as string }
          : undefined) ??
        (pushToTalkReserves(action, binding) ? { id: "pushToTalk" } : undefined);
      if (clash) {
        showNotice(capturing, t("shortcuts.conflict", { action: actionName(clash.id) }));
        return;
      }
      setBinding(capturing, binding === action.defaultBinding ? null : binding);
      setCapturing(null);
    };
    const cancel = () => setCapturing(null);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("pointerdown", cancel, true);
    window.addEventListener("blur", cancel);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("pointerdown", cancel, true);
      window.removeEventListener("blur", cancel);
    };
  }, [capturing, currentBinding, actionName, setBinding, showNotice, t, pttKey, pushToTalk]);

  const overriddenCount = HOTKEY_ACTIONS.filter((a) => bindings[a.id] !== undefined).length;
  const isDesktop = typeof window !== "undefined" && !!window.electron;

  const renderKeys = (binding: string) =>
    bindingKeycaps(binding).map((k, i) => (
      <span className="kbd" key={i}>
        {k}
      </span>
    ));

  return (
    <>
      <h1>{t("shortcuts.title")}</h1>
      <p className="lead">{t("shortcuts.lead")}</p>
      <div className="kbd-toolbar">
        <span className="hint">{t("shortcuts.customizeHint")}</span>
        <button
          type="button"
          className="s-btn secondary"
          onClick={resetAll}
          disabled={overriddenCount === 0}
        >
          {t("shortcuts.resetAll")}
        </button>
      </div>
      {SHORTCUT_GROUPS.map((group) => (
        <div className="s-section" key={group}>
          <h2>{t(`shortcuts.groups.${group}`)}</h2>
          {HOTKEY_ACTIONS.filter((a) => a.group === group).map((action) => {
            const isCapturing = capturing === action.id;
            const overridden = bindings[action.id] !== undefined;
            const binding = currentBinding(action.id);
            const claimer = binding === null ? defaultClaimedBy(bindings, action.id) : null;
            const blocker = binding === null ? null : bindingBlocker(action, binding);
            const hint =
              claimer !== null
                ? t("shortcuts.unboundHint", { action: actionName(claimer) })
                : blocker === null
                  ? t(`shortcuts.hints.${action.id}`, { defaultValue: "" })
                  : blocker.kind === "platform"
                    ? t("shortcuts.unavailableHere")
                    : t("shortcuts.conflict", {
                        action: actionName(blocker.kind === "desktop" ? blocker.id : "pushToTalk"),
                      });
            const active = notice?.id === action.id ? notice.text : hint;
            return (
              <div className={"kbd-row" + (isCapturing ? " capturing" : "")} key={action.id}>
                <div className="kbd-text">
                  <span className="kbd-name">{actionName(action.id)}</span>
                  {active && (
                    <span className={"kbd-hint" + (notice?.id === action.id || blocker !== null || claimer !== null ? " warn" : "")}>
                      {active}
                    </span>
                  )}
                </div>
                <span className="kbd-keys">
                  {overridden && !isCapturing && (
                    <button
                      type="button"
                      className="kbd-reset"
                      title={t("shortcuts.reset")}
                      aria-label={t("shortcuts.reset")}
                      onClick={() => {
                        const fallback = action.defaultBinding;
                        const taken =
                          HOTKEY_ACTIONS.find(
                            (a) => a.id !== action.id && currentBinding(a.id) === fallback,
                          ) ??
                          (window.electron && desktopReservedId(fallback)
                            ? { id: desktopReservedId(fallback) as string }
                            : undefined) ??
                          (pushToTalkReserves(action, fallback)
                            ? { id: "pushToTalk" }
                            : undefined);
                        if (taken) {
                          showNotice(
                            action.id,
                            t("shortcuts.conflict", { action: actionName(taken.id) }),
                          );
                          return;
                        }
                        setBinding(action.id, null);
                      }}
                    >
                      <Icon name="refresh" size={13} />
                    </button>
                  )}
                  <button
                    type="button"
                    className={"kbd-bind" + (isCapturing ? " capturing" : "")}
                    title={t("shortcuts.clickToRebind")}
                    aria-pressed={isCapturing}
                    onClick={() => setCapturing(isCapturing ? null : action.id)}
                  >
                    {isCapturing ? (
                      <span className="kbd kbd-wait">{t("shortcuts.pressKeys")}</span>
                    ) : binding === null ? (
                      <span className="kbd kbd-wait">{t("shortcuts.unbound")}</span>
                    ) : (
                      renderKeys(binding)
                    )}
                  </button>
                </span>
              </div>
            );
          })}
          {group === "voice" && (
            <div className="kbd-row">
              <div className="kbd-text">
                <span className="kbd-name">{actionName("pushToTalk")}</span>
                <span className="kbd-hint">{t("shortcuts.hints.pushToTalk")}</span>
              </div>
              <span className="kbd-keys">
                <button type="button" className="kbd-link" onClick={() => onNavigate("voice")}>
                  {t("shortcuts.openVoice")}
                </button>
                <span className="kbd">{codeLabel(pttKey)}</span>
              </span>
            </div>
          )}
        </div>
      ))}
      {isDesktop && (
        <div className="s-section">
          <h2>{t("shortcuts.groups.app")}</h2>
          {DESKTOP_SHORTCUTS.map((item) => (
            <div className="kbd-row" key={item.id}>
              <div className="kbd-text">
                <span className="kbd-name">{actionName(item.id)}</span>
                <span className="kbd-hint">{t("shortcuts.hints.desktopOnly")}</span>
              </div>
              <span className="kbd-keys">{renderKeys(item.binding)}</span>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
