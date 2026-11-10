/**
 * NEAR's client libraries assume a Node-ish environment: they call
 * `Buffer.from(...)` and read `process.env`. Browsers have neither, and the
 * failure mode is an unhelpful "Cannot read properties of undefined (reading
 * 'from')" at module load — before any of our code runs.
 *
 * This module must be imported before anything that touches near-api-js or the
 * wallet selector.
 */
import { Buffer } from 'buffer';

const globalScope = globalThis as typeof globalThis & {
  Buffer?: typeof Buffer;
  global?: typeof globalThis;
  process?: { env?: Record<string, string>; browser?: boolean };
};

globalScope.Buffer = globalScope.Buffer ?? Buffer;
globalScope.global = globalScope.global ?? globalThis;
globalScope.process = globalScope.process ?? { env: {}, browser: true };
