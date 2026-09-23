const HOP = 128;
const WINDOW = 512;
const MODEL_LOOKBACK = (WINDOW - HOP) / HOP;
const RING_CAPACITY = HOP * 3 * 64;
const DRY_MAX = 8;

class Fir {
  constructor(coefficients) {
    this.coefficients = coefficients;
    this.history = new Float32Array(coefficients.length - 1);
  }

  run(input, output) {
    const taps = this.coefficients.length;
    const history = this.history;
    for (let i = 0; i < input.length; i++) {
      let acc = 0;
      for (let k = 0; k < taps; k++) {
        const index = i - k;
        const sample = index >= 0 ? input[index] : history[history.length + index];
        acc += this.coefficients[k] * sample;
      }
      output[i] = acc;
    }
    const keep = history.length;
    if (input.length >= keep) {
      history.set(input.subarray(input.length - keep));
    } else {
      history.copyWithin(0, input.length);
      history.set(input, keep - input.length);
    }
  }
}

class Delay {
  constructor(samples) {
    this.buffer = new Float32Array(samples);
  }

  run(input, output) {
    const size = this.buffer.length;
    for (let i = 0; i < input.length; i++) {
      output[i] = i < size ? this.buffer[i] : input[i - size];
    }
    if (input.length >= size) {
      this.buffer.set(input.subarray(input.length - size));
    } else {
      this.buffer.copyWithin(0, input.length);
      this.buffer.set(input, size - input.length);
    }
  }
}

class DfnProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.decimation = 3;
    this.block = HOP * this.decimation;
    this.inputBuffer = new Float32Array(this.block);
    this.inputFill = 0;
    this.outputRing = new Float32Array(RING_CAPACITY);
    this.outputRead = 0;
    this.outputWrite = 0;
    this.outputAvailable = 0;
    this.bypass = true;
    this.ready = false;
    this.strength = 1.0;
    this.frames = new Map();
    this.seq = 0;
    this.nextOut = 0;
    this.worker = null;

    this.port.onmessage = (event) => {
      const msg = event.data;
      if (!msg) return;
      if (msg.type === 'init') {
        this.setup(msg);
        return;
      }
      if (msg.type === 'bypass') {
        this.bypass = !!msg.value;
        return;
      }
      if (msg.type === 'strength') {
        const v = Number(msg.value);
        if (Number.isFinite(v)) this.strength = Math.min(1, Math.max(0, v));
        return;
      }
    };
  }

  setup(msg) {
    const coefficients = msg.coefficients;
    this.decimation = msg.decimation || 3;
    this.block = HOP * this.decimation;
    this.inputBuffer = new Float32Array(this.block);
    this.inputFill = 0;
    this.analysisFir = new Fir(coefficients);
    this.synthesisFir = new Fir(coefficients);
    this.dryRefFir = new Fir(coefficients);
    this.dryDelay = new Delay(coefficients.length - 1);
    this.lowAlign = new Delay(this.block * MODEL_LOOKBACK);
    this.highAlign = new Delay(this.block * MODEL_LOOKBACK);
    this.scratchLow = new Float32Array(this.block);
    this.scratchDry = new Float32Array(this.block);
    this.scratchUp = new Float32Array(this.block);
    this.ready = true;
    if (msg.port) {
      this.worker = msg.port;
      this.worker.onmessage = (frameEvent) => {
        const frame = frameEvent.data;
        if (!frame) return;
        if (frame.type === 'frame') this.markWet(frame.seq, frame.payload);
        else if (frame.type === 'drop') this.markDryOnly(frame.seq);
      };
      this.worker.start();
    }
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
      const size = entry.high.length;
      const mixed = new Float32Array(size);
      if (entry.status === 'wet' && entry.wet) {
        const up = this.scratchUp;
        up.fill(0);
        for (let i = 0; i < entry.wet.length; i++) {
          up[i * this.decimation] = entry.wet[i] * this.decimation;
        }
        const wetLow = new Float32Array(size);
        this.synthesisFir.run(up, wetLow);
        const s = this.strength;
        const inv = 1 - s;
        for (let i = 0; i < size; i++) {
          mixed[i] = s * wetLow[i] + inv * entry.low[i] + entry.high[i];
        }
      } else {
        const silence = new Float32Array(size);
        const skipped = new Float32Array(size);
        this.synthesisFir.run(silence, skipped);
        for (let i = 0; i < size; i++) mixed[i] = entry.low[i] + entry.high[i];
      }
      this.enqueueOutput(mixed);
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

  pushBlock() {
    const size = this.block;
    this.analysisFir.run(this.inputBuffer, this.scratchLow);
    this.dryDelay.run(this.inputBuffer, this.scratchDry);

    const payload = new Float32Array(HOP);
    for (let i = 0; i < HOP; i++) {
      payload[i] = this.scratchLow[i * this.decimation];
    }

    const up = this.scratchUp;
    up.fill(0);
    for (let i = 0; i < HOP; i++) {
      up[i * this.decimation] = payload[i] * this.decimation;
    }
    const reference = new Float32Array(size);
    this.dryRefFir.run(up, reference);

    const carried = new Float32Array(size);
    for (let i = 0; i < size; i++) {
      carried[i] = this.scratchDry[i] - reference[i];
    }

    const low = new Float32Array(size);
    const high = new Float32Array(size);
    this.lowAlign.run(reference, low);
    this.highAlign.run(carried, high);

    const seq = this.seq++;
    this.evictOldestPendingIfFull();
    this.frames.set(seq, { low, high, status: 'pending', wet: null });
    if (this.worker) {
      this.worker.postMessage({ type: 'frame', seq, payload }, [payload.buffer]);
    } else {
      this.markDryOnly(seq);
    }
  }

  process(inputs, outputs) {
    const outputChannels = outputs[0];
    if (!outputChannels || outputChannels.length === 0) return true;
    const out = outputChannels[0];
    const inputChannels = inputs[0];

    if (!inputChannels || inputChannels.length === 0) {
      out.fill(0);
      return true;
    }

    const input = inputChannels[0];

    if (this.bypass || !this.ready) {
      out.set(input);
      return true;
    }

    for (let i = 0; i < input.length; i++) {
      this.inputBuffer[this.inputFill++] = input[i];
      if (this.inputFill >= this.block) {
        this.pushBlock();
        this.inputFill = 0;
      }
    }

    this.readOutput(out);
    return true;
  }
}

registerProcessor('dfn-processor', DfnProcessor);
