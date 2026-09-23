import React from "react";
import ProBadge from "../../../subscription/components/ProBadge";
import { useEntitlementsStore } from "../../../subscription/store/entitlementsStore";
import { useTranslation } from "react-i18next";
import { useVoiceSettingsStore } from "../../../voice/store/voiceSettingsStore";
import { usePushToTalkGlobalStatus } from "../../../voice/hooks/usePushToTalkGlobal";
import { activeBindings, effectiveBinding } from "../../lib/hotkeys";
import { HOTKEY_ACTIONS, bindingIsBareKey, codeLabel, keyTokenFromCode } from "../../lib/hotkeyMatching";

import { SettingsRow, Switch } from "../controls";

function MicTestMeter() {
  const micDeviceId = useVoiceSettingsStore((s) => s.micDeviceId);
  const autoSensitivity = useVoiceSettingsStore((s) => s.autoSensitivity);
  const sensitivity = useVoiceSettingsStore((s) => s.sensitivity);
  const micVolume = useVoiceSettingsStore((s) => s.micVolume);
  const [level, setLevel] = React.useState(-120);
  const gainRef = React.useRef<GainNode | null>(null);
  const micVolumeRef = React.useRef(micVolume);
  micVolumeRef.current = micVolume;
  React.useEffect(() => {
    if (gainRef.current) gainRef.current.gain.value = micVolume / 100;
  }, [micVolume]);
  React.useEffect(() => {
    let cancelled = false;
    let stream: MediaStream | null = null;
    let ctx: AudioContext | null = null;
    let timer = 0;
    const start = async () => {
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          audio: {
            ...(micDeviceId ? { deviceId: { exact: micDeviceId } } : {}),
            echoCancellation: true,
            noiseSuppression: true,
            autoGainControl: true,
          },
        });
      } catch {
        return;
      }
      if (cancelled) {
        stream.getTracks().forEach((tr) => tr.stop());
        return;
      }
      let analyser: AnalyserNode;
      let data: Uint8Array;
      try {
        const AudioCtx =
          window.AudioContext ||
          (window as unknown as { webkitAudioContext: typeof AudioContext })
            .webkitAudioContext;
        ctx = new AudioCtx();
        const source = ctx.createMediaStreamSource(stream);
        const gain = ctx.createGain();
        gain.gain.value = micVolumeRef.current / 100;
        gainRef.current = gain;
        analyser = ctx.createAnalyser();
        analyser.fftSize = 1024;
        source.connect(gain);
        gain.connect(analyser);
        data = new Uint8Array(analyser.fftSize);
      } catch {
        stream.getTracks().forEach((tr) => tr.stop());
        ctx?.close().catch(() => {});
        ctx = null;
        return;
      }
      void ctx.resume().catch(() => {});
      const tick = () => {
        if (cancelled) return;
        analyser.getByteTimeDomainData(data);
        let sum = 0;
        for (let i = 0; i < data.length; i++) {
          const d = (data[i] - 128) / 128;
          sum += d * d;
        }
        const db =
          20 * Math.log10(Math.max(Math.sqrt(sum / data.length), 1e-6));
        setLevel((prev) => Math.max(db, prev - 4));
        timer = window.setTimeout(tick, 50);
      };
      tick();
    };
    void start();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      gainRef.current = null;
      stream?.getTracks().forEach((tr) => tr.stop());
      ctx?.close().catch(() => {});
    };
  }, [micDeviceId]);
  const threshold = -72 + 0.72 * (autoSensitivity ? 50 : sensitivity);
  const levelPct = Math.min(100, Math.max(0, ((level + 72) / 72) * 100));
  const thresholdPct = Math.min(100, Math.max(0, ((threshold + 72) / 72) * 100));
  return (
    <div className="mic-test">
      <div
        className={"mic-test-fill" + (level > threshold ? " live" : "")}
        style={{ width: `${levelPct}%` }}
      />
      <div className="mic-test-thr" style={{ left: `${thresholdPct}%` }} />
    </div>
  );
}

const pttKeyLabel = codeLabel;

export function VoiceSection() {
  const { t } = useTranslation("settings");
  const micVolume = useVoiceSettingsStore((s) => s.micVolume);
  const outputVolume = useVoiceSettingsStore((s) => s.outputVolume);
  const noiseMode = useVoiceSettingsStore((s) => s.noiseMode);
  const paid = useEntitlementsStore((s) => s.tier !== "free");
  const { t: tSubscription } = useTranslation("subscription");
  const pttKey = useVoiceSettingsStore((s) => s.pttKey);
  const autoSensitivity = useVoiceSettingsStore((s) => s.autoSensitivity);
  const sensitivity = useVoiceSettingsStore((s) => s.sensitivity);
  const pushToTalk = useVoiceSettingsStore((s) => s.pushToTalk);
  const pttGlobal = useVoiceSettingsStore((s) => s.pttGlobal);
  const setPttGlobal = useVoiceSettingsStore((s) => s.setPttGlobal);
  const screenCodec = useVoiceSettingsStore((s) => s.screenCodec);
  const setScreenCodec = useVoiceSettingsStore((s) => s.setScreenCodec);
  const screenContent = useVoiceSettingsStore((s) => s.screenContent);
  const setScreenContent = useVoiceSettingsStore((s) => s.setScreenContent);
  const globalStatus = usePushToTalkGlobalStatus();
  const globalSupported = globalStatus !== "unsupported";
  const globalFailed = globalStatus === "failed";
  const micDeviceId = useVoiceSettingsStore((s) => s.micDeviceId);
  const speakerDeviceId = useVoiceSettingsStore((s) => s.speakerDeviceId);
  const update = useVoiceSettingsStore((s) => s.update);
  const updateDevices = useVoiceSettingsStore((s) => s.updateDevices);
  const [devices, setDevices] = React.useState<MediaDeviceInfo[]>([]);
  const [capturingKey, setCapturingKey] = React.useState(false);
  const [pttNotice, setPttNotice] = React.useState<string | null>(null);
  const pttNoticeTimer = React.useRef<number | null>(null);
  React.useEffect(
    () => () => {
      if (pttNoticeTimer.current !== null) window.clearTimeout(pttNoticeTimer.current);
    },
    [],
  );
  React.useEffect(() => {
    if (!capturingKey) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (/^(Shift|Control|Alt|Meta)(Left|Right)$/.test(e.code)) {
        setPttNotice(t("voice.pttUnsupported"));
        if (pttNoticeTimer.current !== null) window.clearTimeout(pttNoticeTimer.current);
        pttNoticeTimer.current = window.setTimeout(() => {
          pttNoticeTimer.current = null;
          setPttNotice(null);
        }, 2500);
        setCapturingKey(false);
        return;
      }
      if (e.key !== "Escape" && /^[A-Za-z0-9]+$/.test(e.code)) {
        const token = keyTokenFromCode(e.code);
        const overrides = activeBindings();
        const taken = HOTKEY_ACTIONS.find((a) => {
          const binding = effectiveBinding(overrides, a.id);
          return a.scope === "global" && binding !== null && bindingIsBareKey(binding, token);
        });
        if (taken) {
          setPttNotice(t("shortcuts.conflict", { action: t(`shortcuts.actions.${taken.id}`) }));
          if (pttNoticeTimer.current !== null) window.clearTimeout(pttNoticeTimer.current);
          pttNoticeTimer.current = window.setTimeout(() => {
            pttNoticeTimer.current = null;
            setPttNotice(null);
          }, 2500);
        } else {
          update({ pttKey: e.code });
        }
      }
      setCapturingKey(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [capturingKey, update, t]);
  React.useEffect(() => {
    let cancelled = false;
    const media = navigator.mediaDevices;
    if (!media?.enumerateDevices) return;
    let permissionAsked = false;
    const refresh = async () => {
      try {
        let list = await media.enumerateDevices();
        const unlabeled = list.some((d) => d.kind === "audioinput" && !d.label);
        if (unlabeled && !permissionAsked && media.getUserMedia) {
          permissionAsked = true;
          try {
            const probe = await media.getUserMedia({ audio: true });
            probe.getTracks().forEach((tr) => tr.stop());
            list = await media.enumerateDevices();
          } catch {}
        }
        if (!cancelled) setDevices(list);
      } catch {}
    };
    const onChange = () => {
      void refresh();
    };
    void refresh();
    media.addEventListener?.("devicechange", onChange);
    return () => {
      cancelled = true;
      media.removeEventListener?.("devicechange", onChange);
    };
  }, []);
  const inputs = devices.filter((d) => d.kind === "audioinput");
  const outputs = devices.filter((d) => d.kind === "audiooutput");
  const pct = (v: number) => ({ "--pct": `${v}%` } as React.CSSProperties);
  return (
    <>
      <h1>{t("voice.title")}</h1>
      <p className="lead">{t("voice.lead")}</p>

      <div className="s-section">
        <h2>{t("voice.inputDevice")}</h2>
        <div className="s-field-row">
          <div className="field-lbl">{t("voice.microphone")}</div>
          <select
            className="s-input"
            value={micDeviceId}
            onChange={(e) => updateDevices({ micDeviceId: e.target.value })}
          >
            <option value="">{t("voice.defaultDevice")}</option>
            {inputs.map((d, i) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label || `${t("voice.microphone")} ${i + 1}`}
              </option>
            ))}
          </select>
        </div>
        <SettingsRow label={t("voice.inputVolume")} hint={`${micVolume}%`}>
          <input
            type="range"
            className="s-range"
            min={0}
            max={100}
            value={micVolume}
            style={pct(micVolume)}
            onChange={(e) => update({ micVolume: Number(e.target.value) })}
          />
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("voice.outputDevice")}</h2>
        <div className="s-field-row">
          <div className="field-lbl">{t("voice.speakers")}</div>
          <select
            className="s-input"
            value={speakerDeviceId}
            onChange={(e) => updateDevices({ speakerDeviceId: e.target.value })}
          >
            <option value="">{t("voice.defaultDevice")}</option>
            {outputs.map((d, i) => (
              <option key={d.deviceId} value={d.deviceId}>
                {d.label || `${t("voice.speakers")} ${i + 1}`}
              </option>
            ))}
          </select>
        </div>
        <SettingsRow label={t("voice.outputVolume")} hint={`${outputVolume}%`}>
          <input
            type="range"
            className="s-range"
            min={0}
            max={100}
            value={outputVolume}
            style={pct(outputVolume)}
            onChange={(e) => update({ outputVolume: Number(e.target.value) })}
          />
        </SettingsRow>
      </div>

      <div className="s-section">
        <h2>{t("voice.behavior")}</h2>
        <SettingsRow label={t("voice.pushToTalk")} hint={pttNotice ?? t("voice.pushToTalkHint", { key: pttKeyLabel(pttKey) })}>
          <button
            type="button"
            className={"kbd s-kbd-btn" + (capturingKey ? " capturing" : "")}
            onClick={() => setCapturingKey(true)}
            title={t("voice.pttBindHint")}
          >
            {capturingKey ? t("voice.pttPressKey") : pttKeyLabel(pttKey)}
          </button>
          <Switch on={pushToTalk} onClick={() => update({ pushToTalk: !pushToTalk })}/>
        </SettingsRow>
        <SettingsRow
          label={t("voice.pttGlobal")}
          hint={
            !globalSupported
              ? t("voice.pttGlobalWeb")
              : globalFailed
                ? t("voice.pttGlobalUnavailable")
                : t("voice.pttGlobalHint")
          }
        >
          <Switch
            on={globalSupported && !globalFailed && pttGlobal}
            disabled={!globalSupported || globalFailed || !pushToTalk}
            onClick={() => setPttGlobal(!pttGlobal)}
          />
        </SettingsRow>
        <SettingsRow label={t("voice.screenContent")} hint={t("voice.screenContentHint")}>
          <div className="s-seg">
            <button
              className={screenContent === "auto" ? "on" : ""}
              onClick={() => setScreenContent("auto")}
            >
              {t("voice.screenContentAuto")}
            </button>
            <button
              className={screenContent === "text" ? "on" : ""}
              onClick={() => setScreenContent("text")}
            >
              {t("voice.screenContentText")}
            </button>
            <button
              className={screenContent === "motion" ? "on" : ""}
              onClick={() => setScreenContent("motion")}
            >
              {t("voice.screenContentMotion")}
            </button>
          </div>
        </SettingsRow>
        <SettingsRow label={t("voice.screenCodec")} hint={t("voice.screenCodecHint")}>
          <div className="s-seg">
            <button
              className={screenCodec === "auto" ? "on" : ""}
              onClick={() => setScreenCodec("auto")}
            >
              {t("voice.screenCodecAuto")}
            </button>
            <button
              className={screenCodec === "av1" ? "on" : ""}
              onClick={() => setScreenCodec("av1")}
            >
              AV1
            </button>
            <button
              className={screenCodec === "vp9" ? "on" : ""}
              onClick={() => setScreenCodec("vp9")}
            >
              VP9
            </button>
          </div>
        </SettingsRow>
        <SettingsRow label={t("voice.noiseDefault")} hint={t("voice.noiseDefaultHint")}>
          <div className="s-seg">
            <button className={noiseMode === "off" ? "on" : ""} onClick={() => update({ noiseMode: "off" })}>{t("voice.noiseOff")}</button>
            <button className={noiseMode === "standard" ? "on" : ""} onClick={() => update({ noiseMode: "standard" })}>{t("voice.noiseStandard")}</button>
            <button
              className={noiseMode === "high" && paid ? "on" : ""}
              disabled={!paid}
              title={paid ? undefined : tSubscription("needs.pro")}
              onClick={() => update({ noiseMode: "high" })}
            >
              {t("voice.noiseHigh")}
              {!paid && <ProBadge tier="pro" badgeStyle="tint" size="sm" className="noise-pro-tag" />}
            </button>
          </div>
        </SettingsRow>
        <SettingsRow label={t("voice.autoSensitivity")} hint={t("voice.autoSensitivityHint")}>
          <Switch on={autoSensitivity} onClick={() => update({ autoSensitivity: !autoSensitivity })}/>
        </SettingsRow>
        {!autoSensitivity && (
          <SettingsRow label={t("voice.sensitivity")} hint={`${sensitivity}%`}>
            <input
              type="range"
              className="s-range"
              min={0}
              max={100}
              value={sensitivity}
              style={pct(sensitivity)}
              onChange={(e) => update({ sensitivity: Number(e.target.value) })}
            />
          </SettingsRow>
        )}
        {!autoSensitivity && (
          <div className="s-mictest">
            <div className="s-mictest-head">
              <span>{t("voice.micTest")}</span>
              <span className="s-mictest-hint">{t("voice.micTestHint")}</span>
            </div>
            <MicTestMeter />
          </div>
        )}
      </div>
    </>
  );
}
