export const MIC_GATE_LOOKAHEAD_S = 0.06;
export const MIC_GATE_DENOISED_LOOKAHEAD_S = 0.12;
export const MIC_GATE_TICK_MS = 20;
export const MIC_GATE_HOLD_MS = 300;
export const MIC_GATE_ATTACK_S = 0.004;
export const MIC_GATE_RELEASE_S = 0.03;

const openState = new WeakMap<GainNode, boolean>();

export function isGateOpen(gate: GainNode): boolean {
  return openState.get(gate) ?? gate.gain.value > 0;
}

export function setGateOpen(gate: GainNode, open: boolean): void {
  if (openState.get(gate) === open) return;
  openState.set(gate, open);
  const now = gate.context.currentTime;
  gate.gain.cancelScheduledValues(now);
  gate.gain.setTargetAtTime(open ? 1 : 0, now, open ? MIC_GATE_ATTACK_S : MIC_GATE_RELEASE_S);
}

export function connectThroughLookahead(
  ctx: BaseAudioContext,
  from: AudioNode,
  gate: GainNode,
  lookaheadS: number = MIC_GATE_LOOKAHEAD_S,
): DelayNode {
  const delay = ctx.createDelay(1);
  delay.delayTime.value = lookaheadS;
  from.connect(delay);
  delay.connect(gate);
  return delay;
}
