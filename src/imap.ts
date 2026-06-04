import {
  ImapFlow,
  type FetchMessageObject,
  type FetchQueryObject,
  type SearchObject,
} from 'imapflow';
import { simpleParser } from 'mailparser';
import { startProgress } from './progress.js';
import { config } from './config.js';
import { fileConfig } from './file-config.js';
import type { MailMessage } from './types.js';

// Auto-discovery exclusions, sourced from config.yaml (folders.skipSpecialUse / skipNames).
const SKIP_SPECIAL_USE = new Set(fileConfig.folders.skipSpecialUse);
const SKIP_NAMES = new Set(fileConfig.folders.skipNames); // already lower-cased by the loader

/** Options for read-only folder inspection. */
export interface InspectOptions {
  /** Match subject/from/body (includes already-read mail). */
  search?: string | undefined;
  /** Only unseen messages. */
  unreadOnly?: boolean | undefined;
  /** Only messages received on/after this date (parsed via `new Date`). */
  since?: string | undefined;
  /** Max messages to return (default 30). */
  limit?: number | undefined;
}

/** Operations bound to one currently-open folder. */
export interface FolderSession {
  folder: string;
  listUnread(): Promise<MailMessage[]>;
  /** Read-only: never modifies (fetch uses BODY.PEEK, no flag/move). */
  inspect(opts: InspectOptions): Promise<MailMessage[]>;
  markRead(id: string): Promise<void>;
  moveToDeleted(id: string): Promise<void>;
}

/** Top-level mailbox handle: discover folders and process them one at a time. */
export interface Mailbox {
  /** Selectable folder paths, excluding Sent/Drafts/Trash. */
  discoverFolders(): Promise<string[]>;
  /** Opens `folder`, runs `fn` against it, then releases the lock. */
  process<T>(folder: string, fn: (session: FolderSession) => Promise<T>): Promise<T>;
}

const HTML_NOISE = /<style[\s\S]*?<\/style>|<script[\s\S]*?<\/script>|<[^>]+>/gi;

/** Crude HTML→text fallback for messages that only ship an HTML part. */
function stripHtml(html: string): string {
  return html
    .replace(HTML_NOISE, ' ')
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&#\d+;/g, ' ');
}

interface ParsedBody {
  preview: string;
  hasAttachments: boolean;
}

/**
 * Parses the raw message (mailparser) into a clean text preview + attachment flag.
 * Prefers the plain-text part; falls back to stripped HTML. Robust to malformed mail.
 */
async function parseBody(source: Buffer | undefined): Promise<ParsedBody> {
  if (!source) {
    return { preview: '', hasAttachments: false };
  }
  try {
    const parsed = await simpleParser(source);
    const text =
      parsed.text ?? (typeof parsed.html === 'string' ? stripHtml(parsed.html) : '');
    return {
      preview: text.replace(/\s+/g, ' ').trim().slice(0, fileConfig.imap.bodyPreviewChars),
      hasAttachments: (parsed.attachments?.length ?? 0) > 0,
    };
  } catch {
    return { preview: '', hasAttachments: false };
  }
}

function toIso(value: Date | string | undefined): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'string' && value !== '') {
    return value;
  }
  return new Date(0).toISOString();
}

const FETCH_QUERY: FetchQueryObject = {
  uid: true,
  envelope: true,
  flags: true,
  internalDate: true,
  // Full raw message (BODY.PEEK[] — does NOT set \Seen), parsed by mailparser into
  // clean text + attachment info. Heavier than a snippet, but counts are bounded
  // (unread set / inspect --limit), and it gives readable, decoded bodies.
  source: true,
};

function toMessage(msg: FetchMessageObject, folder: string, body: ParsedBody): MailMessage {
  const from = msg.envelope?.from?.[0];
  return {
    id: String(msg.uid),
    folder,
    subject: msg.envelope?.subject ?? '',
    fromAddress: (from?.address ?? '').toLowerCase(),
    fromName: from?.name ?? '',
    receivedDateTime: toIso(msg.internalDate ?? msg.envelope?.date),
    bodyPreview: body.preview,
    isRead: msg.flags?.has('\\Seen') ?? false,
    hasAttachments: body.hasAttachments,
    importance: 'normal', // IMAP envelope lacks importance; default for the PoC
    internetMessageId: msg.envelope?.messageId ?? '',
  };
}

/** Fetches a UID set or a sequence range into MailMessage objects. Read-only (PEEK). */
async function fetchRange(
  client: ImapFlow,
  folder: string,
  range: string | number[],
  useUid: boolean,
): Promise<MailMessage[]> {
  const messages: MailMessage[] = [];
  const total = Array.isArray(range) ? range.length : undefined;
  const progress = startProgress(`Fetching ${folder}`);
  try {
    for await (const msg of client.fetch(range, FETCH_QUERY, useUid ? { uid: true } : undefined)) {
      const body = await parseBody(msg.source);
      messages.push(toMessage(msg, folder, body));
      progress.tick(messages.length, total);
    }
  } finally {
    progress.stop();
  }
  return messages;
}

/** Reads unseen messages from the currently-open `folder`. */
async function readUnread(client: ImapFlow, folder: string): Promise<MailMessage[]> {
  const uids = await client.search({ seen: false }, { uid: true });
  if (!uids || uids.length === 0) {
    return [];
  }
  return fetchRange(client, folder, uids, true);
}

/**
 * Read-only inspection of the currently-open `folder`. Never modifies anything.
 * - search → match subject/from/body (incl. read mail)
 * - unreadOnly → only unseen
 * - otherwise → the most recent `limit` messages (by sequence, so big folders stay cheap)
 */
async function inspectFolder(
  client: ImapFlow,
  folder: string,
  opts: InspectOptions,
): Promise<MailMessage[]> {
  const limit = opts.limit ?? fileConfig.defaults.inspectLimit;

  // Build an IMAP SEARCH from whichever filters were given (combined with AND).
  const criteria: SearchObject = {};
  if (opts.search) {
    criteria.or = [
      { subject: opts.search },
      { from: opts.search },
      { body: opts.search },
    ];
  }
  if (opts.unreadOnly) {
    criteria.seen = false;
  }
  if (opts.since) {
    criteria.since = new Date(opts.since);
  }

  if (Object.keys(criteria).length > 0) {
    const uids = await client.search(criteria, { uid: true });
    if (!uids || uids.length === 0) {
      return [];
    }
    return fetchRange(client, folder, uids.slice(-limit), true);
  }

  // No filters → the most recent `limit` messages (by sequence, so big folders stay cheap).
  const total = client.mailbox ? client.mailbox.exists : 0;
  if (total === 0) {
    return [];
  }
  const start = Math.max(1, total - limit + 1);
  return fetchRange(client, folder, `${start}:*`, false);
}

/** All selectable folders, minus Sent/Drafts/Trash and non-selectable (\Noselect) ones. */
async function discoverFolders(client: ImapFlow): Promise<string[]> {
  const boxes = await client.list();
  return boxes
    .filter((b) => !b.flags.has('\\Noselect'))
    .filter((b) => !(b.specialUse && SKIP_SPECIAL_USE.has(b.specialUse)))
    .filter((b) => !SKIP_NAMES.has(b.path.toLowerCase()))
    .map((b) => b.path);
}

/** Finds the Trash/Deleted folder via its special-use flag, with a sane fallback. */
async function findTrashPath(client: ImapFlow): Promise<string> {
  const mailboxes = await client.list();
  const trash = mailboxes.find((mb) => mb.specialUse === '\\Trash');
  return trash?.path ?? fileConfig.imap.trashFallback;
}

/**
 * Connects to the mailbox over IMAP (OAuth2 / XOAUTH2), hands a {@link Mailbox} to
 * `fn`, and cleanly logs out afterwards.
 */
export async function withMailbox<T>(
  username: string,
  accessToken: string,
  fn: (mailbox: Mailbox) => Promise<T>,
): Promise<T> {
  const client = new ImapFlow({
    host: config.imapHost,
    port: config.imapPort,
    secure: true,
    auth: { user: username, accessToken },
    logger: false,
  });

  await client.connect();
  try {
    const trashPath = await findTrashPath(client);
    const mailbox: Mailbox = {
      discoverFolders: () => discoverFolders(client),
      process: async (folder, body) => {
        const lock = await client.getMailboxLock(folder);
        try {
          const session: FolderSession = {
            folder,
            listUnread: () => readUnread(client, folder),
            inspect: (opts) => inspectFolder(client, folder, opts),
            markRead: async (id) => {
              await client.messageFlagsAdd(id, ['\\Seen'], { uid: true });
            },
            moveToDeleted: async (id) => {
              // Mark read first so it doesn't linger as unread in Deleted Items.
              await client.messageFlagsAdd(id, ['\\Seen'], { uid: true });
              await client.messageMove(id, trashPath, { uid: true });
            },
          };
          return await body(session);
        } finally {
          lock.release();
        }
      },
    };
    return await fn(mailbox);
  } finally {
    await client.logout();
  }
}
