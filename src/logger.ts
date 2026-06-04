/** Minimal structured-ish console logger. Kept tiny on purpose for the PoC. */

type Level = 'info' | 'warn' | 'error';

function emit(level: Level, message: string): void {
  const line = `[${level.toUpperCase()}] ${message}`;
  if (level === 'error') {
    console.error(line);
  } else if (level === 'warn') {
    console.warn(line);
  } else {
    console.log(line);
  }
}

export const log = {
  info: (message: string): void => emit('info', message),
  warn: (message: string): void => emit('warn', message),
  error: (message: string): void => emit('error', message),
};
