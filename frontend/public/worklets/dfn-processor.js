const HOP = 128;
const RING_CAPACITY = HOP * 64;
const DRY_MAX = 8;

class DfnProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.inputBuffer = new Float32Array(HOP);
    this.inputFill = 0;
    this.outputRing = new Float32Array(RING_CAPACITY);
    this.outputRead = 0;
    this.outputWrite = 0;
    this.outputAvailable = 0;
    this.bypass = true;
    this.strength = 1.0;
    this.frames = new Map();
    this.seq = 0;
    this.nextOut = 0;

    this.port.onmessage = (event) => {
      const msg = event.data;
      if (!msg) return;
      if (msg.type === 'bypass') {
        this.bypass = !!msg.value;
        return;
      }
      if (msg.type === 'strength') {
        const v = Number(msg.value);
        if (Number.isFinite(v)) {
          this.strength = Math.min(1, Math.max(0, v));
        }
        return;
      }
      if (msg.type === 'frame' && msg.payload instanceof Float32Array) {
        this.markWet(msg.seq, msg.payload);
        return;
      }
      if (msg.type === 'drop') {
        this.markDryOnly(msg.seq);
        return;
      }
    };
  }

  markWet(seq, wet) {
    const entry = this.frames.get(seq);
    if (!entry || entry.status !== 'pending') return;
    entry.status = 'wet';
    entry.wet = wet;
    this.drain();
  }

  markDryOnly(seq) {
    const entry = this.frames.get(seq);
    if (!entry || entry.status !== 'pending') return;
    entry.status = 'dry';
    this.drain();
  }

  drain() {
    while (true) {
      const entry = this.frames.get(this.nextOut);
      if (!entry || entry.status === 'pending') return;
      this.frames.delete(this.nextOut);
      this.nextOut += 1;
      if (entry.status === 'wet' && entry.wet) {
        const mixed = new Float32Array(HOP);
        const s = this.strength;
        const inv = 1 - s;
        for (let i = 0; i < HOP; i++) {
          mixed[i] = inv * entry.dry[i] + s * entry.wet[i];
        }
        this.enqueueOutput(mixed);
      } else {
        this.enqueueOutput(entry.dry);
      }
    }
  }

  enqueueOutput(frame) {
    for (let i = 0; i < frame.length; i++) {
      this.outputRing[this.outputWrite] = frame[i];
      this.outputWrite = (this.outputWrite + 1) % RING_CAPACITY;
      if (this.outputAvailable < RING_CAPACITY) {
        this.outputAvailable += 1;
      } else {
        this.outputRead = (this.outputRead + 1) % RING_CAPACITY;
      }
    }
  }

  readOutput(out) {
    for (let i = 0; i < out.length; i++) {
      if (this.outputAvailable === 0) {
        out[i] = 0;
      } else {
        out[i] = this.outputRing[this.outputRead];
        this.outputRead = (this.outputRead + 1) % RING_CAPACITY;
        this.outputAvailable -= 1;
      }
    }
  }

  evictOldestPendingIfFull() {
    if (this.frames.size < DRY_MAX) return;
    for (const entry of this.frames.values()) {
      if (entry.status === 'pending') {
        entry.status = 'dry';
        this.drain();
        return;
      }
    }
  }

  process(inputs, outputs) {
    const inputChannels = inputs[0];
    const outputChannels = outputs[0];
    if (!outputChannels || outputChannels.length === 0) return true;

    const out = outputChannels[0];

    if (!inputChannels || inputChannels.length === 0) {
      out.fill(0);
      return true;
    }

    const input = inputChannels[0];

    if (this.bypass) {
      out.set(input);
      return true;
    }

    for (let i = 0; i < input.length; i++) {
      this.inputBuffer[this.inputFill++] = input[i];
      if (this.inputFill >= HOP) {
        this.evictOldestPendingIfFull();
        const dry = new Float32Array(HOP);
        dry.set(this.inputBuffer);
        const seq = this.seq++;
        this.frames.set(seq, { dry, status: 'pending', wet: null });
        const forSend = new Float32Array(HOP);
        forSend.set(this.inputBuffer);
        this.port.postMessage(
          { type: 'frame', seq, payload: forSend },
          [forSend.buffer],
        );
        this.inputFill = 0;
      }
    }

    this.readOutput(out);
    return true;
  }
}

registerProcessor('dfn-processor', DfnProcessor);
