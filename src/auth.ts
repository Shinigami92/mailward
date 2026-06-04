import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { createInterface } from 'node:readline/promises';
import {
  CryptoProvider,
  LogLevel,
  PublicClientApplication,
  type Configuration,
  type ICachePlugin,
  type TokenCacheContext,
} from '@azure/msal-node';
import { env } from './env.js';
import { log } from './logger.js';
import type { ResolvedAccount } from './accounts.js';

/** Set MSAL_DEBUG=1 to see MSAL's auth/token traffic (helpful for diagnosing hangs). */
const debugEnabled = env.MSAL_DEBUG;

/** Thrown when an account's auth method has no implementation yet → orchestrator skips it. */
export class VendorNotImplementedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'VendorNotImplementedError';
  }
}

/** IMAP credentials for one account: XOAUTH2 access token, or a password. */
export interface Credentials {
  user: string;
  accessToken?: string;
  pass?: string;
}

/**
 * Per-account MSAL cache plugin. Persists the token cache (incl. the refresh token) to a file in
 * the account's cache directory so later runs acquire tokens silently. NOTE: this file contains
 * credentials; it lives under the gitignored cache dir.
 */
function cachePlugin(cacheFile: string): ICachePlugin {
  return {
    async beforeCacheAccess(ctx: TokenCacheContext): Promise<void> {
      try {
        ctx.tokenCache.deserialize(await readFile(cacheFile, 'utf8'));
      } catch {
        // No cache yet — first run for this account. Nothing to load.
      }
    },
    async afterCacheAccess(ctx: TokenCacheContext): Promise<void> {
      if (!ctx.cacheHasChanged) {
        return;
      }
      await mkdir(dirname(cacheFile), { recursive: true });
      await writeFile(cacheFile, ctx.tokenCache.serialize(), 'utf8');
    },
  };
}

function buildPca(account: ResolvedAccount): PublicClientApplication {
  const msalConfig: Configuration = {
    auth: {
      clientId: account.clientId ?? '',
      authority: account.authority,
    },
    cache: { cachePlugin: cachePlugin(join(account.cacheDir, 'msal.json')) },
    system: debugEnabled
      ? {
          loggerOptions: {
            loggerCallback: (_level, message) => log.info(`msal[${account.id}]: ${message}`),
            piiLoggingEnabled: false,
            logLevel: LogLevel.Verbose,
          },
        }
      : undefined,
  };
  return new PublicClientApplication(msalConfig);
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

/**
 * Interactive authorization-code flow with manual code paste (no local server).
 * Used on the first run; the resulting refresh token is cached for silent reuse.
 */
async function interactiveSignIn(
  account: ResolvedAccount,
  pca: PublicClientApplication,
  scopes: string[],
): Promise<Credentials> {
  const crypto = new CryptoProvider();
  const { verifier, challenge } = await crypto.generatePkceCodes();

  const authUrl = await pca.getAuthCodeUrl({
    scopes,
    redirectUri: account.redirectUri ?? '',
    codeChallenge: challenge,
    codeChallengeMethod: 'S256',
    prompt: 'select_account',
  });

  log.info(`[${account.id}] 1) Open this URL in your browser and sign in / consent:`);
  log.info(authUrl);
  log.info(
    '2) You will be redirected to a https://localhost/... page that fails to load — that is expected.',
  );
  log.info('3) Copy the FULL address bar URL and paste it below.');

  const rl = createInterface({ input: process.stdin, output: process.stdout });
  const pasted = await rl.question(`[${account.id}] Paste redirected URL (or just the code): `);
  rl.close();

  const result = await pca.acquireTokenByCode({
    scopes,
    redirectUri: account.redirectUri ?? '',
    code: extractAuthCode(pasted),
    codeVerifier: verifier,
  });

  if (!result?.accessToken || !result.account) {
    throw new Error('Authorization-code flow did not return an access token.');
  }
  return { user: result.account.username, accessToken: result.accessToken };
}

/** MSAL XOAUTH2: silent from the per-account cache, else interactive sign-in. */
async function msalAuth(account: ResolvedAccount): Promise<Credentials> {
  const scopes = [...(account.scopes ?? [])];
  const pca = buildPca(account);
  const cache = pca.getTokenCache();
  const cached = (await cache.getAllAccounts())[0];

  if (cached) {
    try {
      const silent = await pca.acquireTokenSilent({ account: cached, scopes });
      if (silent?.accessToken) {
        return { user: cached.username, accessToken: silent.accessToken };
      }
    } catch {
      log.warn(`[${account.id}] Silent token acquisition failed; falling back to sign-in.`);
    }
  }
  return interactiveSignIn(account, pca, scopes);
}

/**
 * Authenticates one account and returns IMAP credentials. Microsoft (xoauth2-msal) is wired;
 * other auth methods are prepared but not implemented and throw {@link VendorNotImplementedError}
 * so the orchestrator can skip them with a warning.
 */
export async function authenticate(account: ResolvedAccount): Promise<Credentials> {
  switch (account.authMethod) {
    case 'xoauth2-msal':
      return msalAuth(account);
    case 'password':
      throw new VendorNotImplementedError(
        `password / app-password auth (vendor "${account.vendor}") is not implemented yet`,
      );
    case 'xoauth2-google':
      throw new VendorNotImplementedError(
        `Google OAuth (vendor "${account.vendor}") is not implemented yet`,
      );
    default:
      throw new VendorNotImplementedError(`auth method "${account.authMethod}" is not implemented`);
  }
}
