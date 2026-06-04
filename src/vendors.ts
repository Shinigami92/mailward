/**
 * Vendor registry: per-provider connection + auth defaults. An account in accounts.yaml
 * picks a `vendor` and inherits these defaults; any field can be overridden per account.
 *
 * Only the `xoauth2-msal` auth method is wired today (Microsoft/Outlook personal accounts).
 * `password` (app-password / generic IMAP) and `xoauth2-google` are declared but not yet
 * implemented — accounts using them are skipped with a warning (see src/auth.ts).
 */

export type AuthMethod = 'xoauth2-msal' | 'xoauth2-google' | 'password';

export interface VendorDefaults {
  imapHost: string;
  imapPort: number;
  authMethod: AuthMethod;
  /** OAuth (MSAL) specifics — only meaningful for xoauth2-msal. */
  clientId?: string;
  authority?: string;
  redirectUri?: string;
  scopes?: string[];
}

export interface Vendor {
  defaults: VendorDefaults;
}

/**
 * Mozilla Thunderbird's public OAuth client ID. Personal Microsoft accounts can no longer
 * register their own app without an Entra directory (paid/gated), so we reuse this well-known
 * public client — the same one Thunderbird and mutt_oauth2.py use. Consequence: the first-run
 * consent screen shows "Mozilla Thunderbird". Override via an account's `clientId` if you ever
 * register your own app.
 */
const THUNDERBIRD_CLIENT_ID = '9e5f94bc-e8a4-4e73-b8be-63364c29d753';

export const VENDORS: Record<string, Vendor> = {
  microsoft: {
    defaults: {
      imapHost: 'outlook.office365.com',
      imapPort: 993,
      authMethod: 'xoauth2-msal',
      clientId: THUNDERBIRD_CLIENT_ID,
      // "common" (not "consumers") is required for the borrowed multi-tenant client to
      // accept a personal account through the auth-code flow.
      authority: 'https://login.microsoftonline.com/common',
      redirectUri: 'https://localhost',
      scopes: ['https://outlook.office.com/IMAP.AccessAsUser.All'],
    },
  },
  gmail: {
    defaults: {
      imapHost: 'imap.gmail.com',
      imapPort: 993,
      // App-password over plain IMAP for now; a Google-OAuth (xoauth2-google) path can be
      // added later. Either way, auth is not implemented yet (prepared, not wired).
      authMethod: 'password',
    },
  },
  generic: {
    defaults: {
      // No sensible host default — a generic IMAP account must set imapHost itself.
      imapHost: '',
      imapPort: 993,
      authMethod: 'password',
    },
  },
};

export function getVendor(name: string): Vendor | undefined {
  return VENDORS[name];
}

export const VENDOR_NAMES = Object.keys(VENDORS);
