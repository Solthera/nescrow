import type { U128 } from '../types/contract';

const YOCTO_PER_NEAR = 10n ** 24n;

/** 1234567890000000000000000n -> "1.2345" */
export function formatNear(yocto: U128 | bigint, maxDecimals = 4): string {
  const value = typeof yocto === 'bigint' ? yocto : BigInt(yocto || '0');
  const negative = value < 0n;
  const abs = negative ? -value : value;
  const whole = abs / YOCTO_PER_NEAR;
  const fraction = abs % YOCTO_PER_NEAR;

  let text = whole.toString();
  if (fraction > 0n && maxDecimals > 0) {
    const digits = fraction.toString().padStart(24, '0').slice(0, maxDecimals);
    const trimmed = digits.replace(/0+$/, '');
    if (trimmed) text += `.${trimmed}`;
  }
  return negative ? `-${text}` : text;
}

/** Fixed decimals, for money that must not jitter: 2000000000000000000000000n -> "2.0000" */
export function formatNearFixed(yocto: U128 | bigint, decimals = 4): string {
  const value = typeof yocto === 'bigint' ? yocto : BigInt(yocto || '0');
  const whole = value / YOCTO_PER_NEAR;
  const fraction = (value % YOCTO_PER_NEAR).toString().padStart(24, '0').slice(0, decimals);
  return decimals > 0 ? `${whole}.${fraction}` : whole.toString();
}

/** "1.5" -> 1500000000000000000000000n. Rejects anything that is not a plain
 * decimal with at most 24 fractional digits — no parseFloat, ever. */
export function parseNearToYocto(input: string): bigint {
  const text = input.trim();
  if (!/^\d+(\.\d{1,24})?$/.test(text)) {
    throw new Error('Enter a positive amount with at most 24 decimal places.');
  }
  const [whole, fraction = ''] = text.split('.');
  return BigInt(whole) * YOCTO_PER_NEAR + BigInt(fraction.padEnd(24, '0'));
}

export function shortAddress(address: string, keep = 6): string {
  if (address.length <= keep * 2 + 3) return address;
  return `${address.slice(0, keep)}…${address.slice(-keep)}`;
}

export function nowNanos(): bigint {
  return BigInt(Date.now()) * 1_000_000n;
}
