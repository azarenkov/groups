const RING_CAPACITY = 48000;
const PENDING_MAX = 8;
const PRIME_FRAMES = 2;
const EXCESS_FRAMES = 4;

class Delay {
  constructor(samples) {
    this.buffer = new Float32Array(samples);
  }

  run(input, output) {
    const size = this.buffer.length;
    if (size === 0) {
      output.set(input);
      return;
    }
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
    this.ready = false;
    this.frameLength = 0;
    this.inputBuffer = null;
    this.inputFill = 0;
    this.dryDelay = null;
    this.outputRing = new Float32Array(RING_CAPACITY);
    this.outputRead = 0;
    this.outputWrite = 0;
    this.outputAvailable = 0;
    this.prime = 0;
    this.maxBuffered = 0;
    this.primed = false;
    this.frames = new Map();
    this.seq = 0;
    this.nextOut = 0;
    this.worker = null;

    this.port.onmessage = (event) => {
      const msg = event.data;
      if (msg && msg.type === 'init') this.setup(msg);
    };
  }

  setup(msg) {
    this.frameLength = msg.frameLength;
    this.inputBuffer = new Float32Array(this.frameLength);
    this.inputFill = 0;
    this.dryDelay = new Delay(msg.latency || 0);
    this.prime = this.frameLength * PRIME_FRAMES;
    this.maxBuffered = this.prime + this.frameLength * EXCESS_FRAMES;
    this.primed = false;
    this.worker = msg.port;
    this.worker.onmessage = (frameEvent) => {
      const frame = frameEvent.data;
      if (!frame) return;
      if (frame.type === 'frame') this.markWet(frame.seq, frame.payload);
      else if (frame.type === 'drop') this.markDry(frame.seq);
    };
    this.worker.start();
    this.ready = true;
  }

  markWet(seq, wet) {
    const entry = this.frames.get(seq);
    if (!entry || entry.status !== 'pending') return;
    entry.status = 'wet';
    entry.wet = wet;
    this.drain();
  }

  markDry(seq) {
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
      this.enqueueOutput(entry.status === 'wet' && entry.wet ? entry.wet : entry.dry);
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
    const excess = this.outputAvailable - this.maxBuffered;
    if (excess > 0) {
      this.outputRead = (this.outputRead + excess) % RING_CAPACITY;
      this.outputAvailable -= excess;
    }
  }

  readOutput(out) {
    if (!this.primed) {
      if (this.outputAvailable < this.prime) {
        out.fill(0);
        return;
      }
      this.primed = true;
    }
    for (let i = 0; i < out.length; i++) {
      if (this.outputAvailable === 0) {
        out.fill(0, i);
        this.primed = false;
        return;
      }
      out[i] = this.outputRing[this.outputRead];
      this.outputRead = (this.outputRead + 1) % RING_CAPACITY;
      this.outputAvailable -= 1;
    }
  }

  evictOldestPendingIfFull() {
    if (this.frames.size < PENDING_MAX) return;
    for (const entry of this.frames.values()) {
      if (entry.status === 'pending') {
        entry.status = 'dry';
        this.drain();
        return;
      }
    }
  }

  pushFrame() {
    const dry = new Float32Array(this.frameLength);
    this.dryDelay.run(this.inputBuffer, dry);
    const payload = this.inputBuffer.slice();
    const seq = this.seq++;
    this.evictOldestPendingIfFull();
    this.frames.set(seq, { dry, status: 'pending', wet: null });
    this.worker.postMessage({ type: 'frame', seq, payload }, [payload.buffer]);
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

    if (!this.ready) {
      out.set(input);
      return true;
    }

    for (let i = 0; i < input.length; i++) {
      this.inputBuffer[this.inputFill++] = input[i];
      if (this.inputFill >= this.frameLength) {
        this.pushFrame();
        this.inputFill = 0;
      }
    }

    this.readOutput(out);
    return true;
  }
}

registerProcessor('dfn-processor', DfnProcessor);
