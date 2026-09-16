import React from "react";

export interface SwitchProps {
  disabled?: boolean; on: boolean; onClick: () => void; }

export function Switch({ on, onClick, disabled }: SwitchProps) {
  return <button className={'s-switch' + (on ? ' on' : '')} onClick={onClick} disabled={disabled} aria-pressed={on}/>;
}

export interface SettingsRowProps {
  label: string;
  hint?: string;
  children: React.ReactNode;
}

export function SettingsRow({ label, hint, children }: SettingsRowProps) {
  return (
    <div className="s-row">
      <div className="s-row-text">
        <div className="lbl">{label}</div>
        {hint && <div className="hint">{hint}</div>}
      </div>
      <div className="s-row-control">{children}</div>
    </div>
  );
}

export const ACCENT_OPTIONS = [
  { name: 'Blue',     value: '#0071e3' },
  { name: 'Sage',     value: '#5b7e5e' },
  { name: 'Indigo',   value: '#5b6cd9' },
  { name: 'Coral',    value: '#d97557' },
  { name: 'Amber',    value: '#c89a3d' },
  { name: 'Plum',     value: '#8a5b9e' },
  { name: 'Slate',    value: '#5a6470' },
];
