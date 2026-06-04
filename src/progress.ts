/**
 * Tiny in-place progress indicator. Renders to stderr and ONLY when stderr is a TTY,
 * so piped/redirected output (JSON, tables) stays clean. Refreshes a single line.
 */

const isTty = process.stderr.isTTY === true;
const BAR_WIDTH = 24;

export interface Progress {
  tick(done: number, total?: number): void;
  stop(): void;
}

const noop: Progress = { tick: () => {}, stop: () => {} };

export function startProgress(label: string): Progress {
  if (!isTty) {
    return noop;
  }
  return {
    tick(done, total) {
      let body: string;
      if (total && total > 0) {
        const ratio = Math.min(1, done / total);
        const filled = Math.round(ratio * BAR_WIDTH);
        body = `[${'█'.repeat(filled)}${'░'.repeat(BAR_WIDTH - filled)}] ${done}/${total}`;
      } else {
        body = `… ${done}`;
      }
      // \r → line start, then clear-to-end (\x1b[K) removes any leftover from a longer line.
      process.stderr.write(`\r${label} ${body}\x1b[K`);
    },
    stop() {
      process.stderr.write('\r\x1b[K');
    },
  };
}
