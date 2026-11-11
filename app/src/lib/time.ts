/** Chain timestamps are nanoseconds as strings; never Number() them whole. */
export function formatStamp(nanos: string | null | undefined): string {
  if (!nanos || nanos === '0') return '—';
  const ms = Number(BigInt(nanos) / 1_000_000n);
  const d = new Date(ms);
  const day = d.toLocaleDateString(undefined, { day: '2-digit', month: 'short' });
  const time = d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
  return `${day} ${time}`;
}

export interface Countdown {
  label: string;
  text: string;
  /** 0 = just started, 1 = expired. */
  ratio: number;
  expired: boolean;
}

/** Remaining time for a window that started at `start` and closes at `end`. */
export function countdown(startNanos: string, endNanos: string, nowNanos: bigint): Countdown {
  const start = BigInt(startNanos || '0');
  const end = BigInt(endNanos || '0');
  const now = nowNanos;

  if (end === 0n) return { label: '', text: '—', ratio: 0, expired: false };

  const total = end > start ? end - start : 0n;
  const leftNs = end > now ? end - now : 0n;
  const ratio = total > 0n ? Number(((total - leftNs) * 1000n) / total) / 1000 : 1;
  const expired = leftNs === 0n;

  const minutes = Number(leftNs / 60_000_000_000n);
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor((minutes % 1440) / 60);
  const mins = minutes % 60;

  const text = expired
    ? 'expired'
    : days > 0
      ? `${days}d ${hours}h`
      : hours > 0
        ? `${hours}h ${mins}m`
        : `${mins}m`;

  return { label: '', text, ratio: Math.min(1, Math.max(0, ratio)), expired };
}
