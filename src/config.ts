import { env } from './env.js';

export const config = {
  clientId: env.CLIENT_ID,
  authority: env.AUTHORITY,
  redirectUri: env.REDIRECT_URI,
  imapHost: env.IMAP_HOST,
  imapPort: env.IMAP_PORT,
  /**
   * Outlook IMAP scope (the Thunderbird client ID is entitled to Outlook scopes,
   * not Microsoft Graph). offline_access/openid/profile are added by MSAL.
   */
  scopes: ['https://outlook.office.com/IMAP.AccessAsUser.All'],
  /** Where the MSAL token cache (refresh token etc.) is persisted. */
  tokenCachePath: env.TOKEN_CACHE_PATH,
} as const;
