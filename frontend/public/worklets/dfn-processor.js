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
    this.dryMap = new Map();
    this.dryOrder = [];
    this.seq = 0;

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
        this.handleProcessedFrame(msg.seq, msg.payload);
        return;
      }
      if (msg.type === 'drop') {
        this.handleDroppedFrame(msg.seq);
        return;
      }
    };
  }

  takeDry(seq) {
    const dry = this.dryMap.get(seq);
    if (!dry) return null;
    this.dryMap.delete(seq);
    const idx = this.dryOrder.indexOf(seq);
    if (idx >= 0) this.dryOrder.splice(idx, 1);
    return dry;
  }

  handleDroppedFrame(seq) {
    const dry = this.takeDry(seq);
    if (dry) this.enqueueOutput(dry);
  }

  handleProcessedFrame(seq, wet) {
    const dry = this.takeDry(seq);
    const mixed = new Float32Array(HOP);
    if (dry) {
      const s = this.strength;
      const inv = 1 - s;
      for (let i = 0; i < HOP; i++) {
        mixed[i] = inv * dry[i] + s * wet[i];
      }
    } else {
      mixed.set(wet);
    }
    this.enqueueOutput(mixed);
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
        const dry = new Float32Array(HOP);
        dry.set(this.inputBuffer);
        if (this.dryOrder.length >= DRY_MAX) {
          const oldestSeq = this.dryOrder.shift();
          const stale = this.dryMap.get(oldestSeq);
          this.dryMap.delete(oldestSeq);
          if (stale) this.enqueueOutput(stale);
        }
        const seq = this.seq++;
        this.dryMap.set(seq, dry);
        this.dryOrder.push(seq);
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
