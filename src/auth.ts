import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { createInterface } from 'node:readline/promises';
import {
  CryptoProvider,
  LogLevel,
  PublicClientApplication,
  type Configuration,
  type ICachePlugin,
  type TokenCacheContext,
} from '@azure/msal-node';
import { config } from './config.js';
import { env } from './env.js';
import { log } from './logger.js';

/** Set MSAL_DEBUG=1 to see MSAL's auth/token traffic (helpful for diagnosing hangs). */
const debugEnabled = env.MSAL_DEBUG;

/**
 * Persists the MSAL token cache (incl. the refresh token) to disk so that, after
 * one interactive device-code sign-in, later runs acquire tokens silently — the
 * key to unattended cron/daemon operation.
 *
 * NOTE: this file contains credentials. It lives under .cache/ which is gitignored.
 */
const filePlugin: ICachePlugin = {
  async beforeCacheAccess(ctx: TokenCacheContext): Promise<void> {
    try {
      const data = await readFile(config.tokenCachePath, 'utf8');
      ctx.tokenCache.deserialize(data);
    } catch {
      // No cache yet — first run. Nothing to load.
    }
  },
  async afterCacheAccess(ctx: TokenCacheContext): Promise<void> {
    if (!ctx.cacheHasChanged) {
      return;
    }
    await mkdir(dirname(config.tokenCachePath), { recursive: true });
    await writeFile(config.tokenCachePath, ctx.tokenCache.serialize(), 'utf8');
  },
};

let cachedPca: PublicClientApplication | undefined;

/** Builds the MSAL app lazily so importing this module doesn't require config. */
function getPca(): PublicClientApplication {
  if (!cachedPca) {
    const msalConfig: Configuration = {
      auth: {
        clientId: config.clientId,
        authority: config.authority,
      },
      cache: { cachePlugin: filePlugin },
      system: debugEnabled
        ? {
            loggerOptions: {
              loggerCallback: (_level, message) => log.info(`msal: ${message}`),
              piiLoggingEnabled: false,
              logLevel: LogLevel.Verbose,
            },
          }
        : undefined,
    };
    cachedPca = new PublicClientApplication(msalConfig);
  }
  return cachedPca;
}

/** Extracts the `code` from a pasted redirect URL, or accepts a bare code. */
function extractAuthCode(pasted: string): string {
  const trimmed = pasted.trim();
  try {
    const url = new URL(trimmed);
    const error = url.searchParams.get('error');
    if (error) {
      throw new Error(
        `Authorization failed: ${error} — ${url.searchParams.get('error_description') ?? ''}`,
      );
    }
    const code = url.searchParams.get('code');
    if (code) {
      return code;
    }
  } catch {
    // Not a URL — fall through and treat the input as a raw code.
  }
  if (trimmed === '') {
    throw new Error('No authorization code provided.');
  }
  return trimmed;
}

/** Access token plus the signed-in account's username (the mailbox address). */
export interface Auth {
  accessToken: string;
  username: string;
}

/**
 * Interactive authorization-code flow with manual code paste (no local server).
 * Used on the first run; the resulting refresh token is cached for silent reuse.
 */
async function interactiveSignIn(
  pca: PublicClientApplication,
  scopes: string[],
): Promise<Auth> {
  const crypto = new CryptoProvider();
  const { verifier, challenge } = await crypto.generatePkceCodes();

  const authUrl = await pca.getAuthCodeUrl({
    scopes,
    redirectUri: config.redirectUri,
    codeChallenge: challenge,
    codeChallengeMethod: 'S256',
    prompt: 'select_account',
  });

  log.info('1) Open this URL in your browser and sign in / consent:');
  log.info(authUrl);
  log.info(
    '2) You will be redirected to a https://localhost/... page that fails to load — that is expected.',
  );
  log.info('3) Copy the FULL address bar URL and paste it below.');

  const rl = createInterface({ input: process.stdin, output: process.stdout });
  const pasted = await rl.question('Paste redirected URL (or just the code): ');
  rl.close();

  const result = await pca.acquireTokenByCode({
    scopes,
    redirectUri: config.redirectUri,
    code: extractAuthCode(pasted),
    codeVerifier: verifier,
  });

  if (!result?.accessToken || !result.account) {
    throw new Error('Authorization-code flow did not return an access token.');
  }
  return { accessToken: result.accessToken, username: result.account.username };
}

/**
 * Returns an access token + mailbox username. Tries the persisted cache first
 * (silent, no prompt); falls back to interactive sign-in on the first run or when
 * the refresh token has expired.
 */
export async function getAuth(): Promise<Auth> {
  const pca = getPca();
  const scopes = [...config.scopes];
  const cache = pca.getTokenCache();
  const accounts = await cache.getAllAccounts();
  const account = accounts[0];

  if (account) {
    try {
      const silent = await pca.acquireTokenSilent({ account, scopes });
      if (silent?.accessToken) {
        return { accessToken: silent.accessToken, username: account.username };
      }
    } catch {
      log.warn('Silent token acquisition failed; falling back to interactive sign-in.');
    }
  }

  return interactiveSignIn(pca, scopes);
}
