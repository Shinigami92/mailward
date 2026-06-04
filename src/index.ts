import { writeFile } from 'node:fs/promises';
import { parseArgs } from 'node:util';
import { authenticate, VendorNotImplementedError } from './auth.js';
import { accounts, type ResolvedAccount } from './accounts.js';
import {
  withMailbox,
  type FolderSession,
  type Mailbox,
  type MailboxConnection,
} from './imap.js';
import { RuleClassifier, ageInHours } from './classifier/rule-classifier.js';
import { rules } from './rules.config.js';
import { cleanupRules, expirySignal } from './cleanup.config.js';
import { isConfidential, isIgnored } from './folders.config.js';
import { fileConfig } from './file-config.js';
import { toCsv } from './csv.js';
import { markdownTable, truncate } from './table.js';
import { log } from './logger.js';
import type { Classifier, Decision, MailMessage } from './types.js';

interface Cli {
  apply: boolean;
  limit: number | undefined;
  export: string | undefined;
  inspect: boolean;
  cleanup: boolean;
  folder: string | undefined;
  search: string | undefined;
  since: string | undefined;
  unread: boolean;
  account: string | undefined;
}

function parseCli(): Cli {
  const { values } = parseArgs({
    options: {
      apply: { type: 'boolean', default: false },
      limit: { type: 'string' },
      export: { type: 'string' },
      inspect: { type: 'boolean', default: false },
      cleanup: { type: 'boolean', default: false },
      folder: { type: 'string' },
      search: { type: 'string' },
      since: { type: 'string' },
      unread: { type: 'boolean', default: false },
      account: { type: 'string' },
    },
  });
  const limit = values.limit ? Number.parseInt(values.limit, 10) : undefined;
  return {
    apply: values.apply ?? false,
    limit,
    export: values.export,
    inspect: values.inspect ?? false,
    cleanup: values.cleanup ?? false,
    folder: values.folder,
    search: values.search,
    since: values.since,
    unread: values.unread ?? false,
    account: values.account,
  };
}

function ageInDays(message: MailMessage): number {
  return Math.floor(ageInHours(message) / 24);
}

/** "YYYY-MM-DD (Nd)" date+age column used in every results table. */
function dateCol(m: MailMessage): string {
  return `${m.receivedDateTime.slice(0, 10)} (${ageInDays(m)}d)`;
}

/** Most recent first. receivedDateTime is ISO-8601, so a string compare is chronological. */
function byNewestFirst(a: MailMessage, b: MailMessage): number {
  return b.receivedDateTime.localeCompare(a.receivedDateTime);
}

type Tally = Record<Decision, number>;

interface ActionRecord {
  message: MailMessage;
  decision: Exclude<Decision, 'keep'>;
  reason: string;
  ok: boolean;
}

/** Mutable run state threaded across folders so --limit and the summary are global. */
interface RunState {
  cli: Cli;
  classifier: Classifier;
  tally: Tally;
  scanned: number;
  actions: ActionRecord[];
}

async function applyDecision(
  session: FolderSession,
  decision: Exclude<Decision, 'keep'>,
  id: string,
): Promise<void> {
  if (decision === 'markRead') {
    await session.markRead(id);
  } else {
    await session.moveToDeleted(id);
  }
}

async function processFolder(session: FolderSession, state: RunState): Promise<void> {
  let messages = await session.listUnread();
  if (state.cli.limit !== undefined) {
    messages = messages.slice(0, Math.max(0, state.cli.limit - state.scanned));
  }
  if (messages.length === 0) {
    return;
  }
  state.scanned += messages.length;

  for (const message of messages) {
    const { decision, reason } = state.classifier.classify(message);
    state.tally[decision] += 1;
    if (decision === 'keep') {
      continue;
    }

    let ok = true;
    if (state.cli.apply) {
      try {
        await applyDecision(session, decision, message.id);
      } catch (error) {
        ok = false;
        log.error(
          `Failed to ${decision} [${message.folder}] "${message.subject}": ${String(error)}`,
        );
      }
    }
    state.actions.push({ message, decision, reason, ok });
  }
}

/** Reads (without acting on) all unread messages across the given folders. */
async function collectUnread(mailbox: Mailbox, folders: string[]): Promise<MailMessage[]> {
  const all: MailMessage[] = [];
  for (const folder of folders) {
    try {
      await mailbox.process(folder, async (session) => {
        all.push(...(await session.listUnread()));
      });
    } catch (error) {
      log.warn(`Skipping folder "${folder}": ${String(error)}`);
    }
  }
  return all;
}

const EXPORT_COLUMNS = [
  'account',
  'folder',
  'fromName',
  'fromAddress',
  'subject',
  'receivedDateTime',
  'internetMessageId',
  'decision', // <- you fill these two: keep | markRead | delete
  'reason',
] as const;

/**
 * Folders for the default auto-scan. Always drops `ignore` folders; drops `confidential`
 * ones too unless the user listed folders explicitly (opt-in via the scan list).
 */
async function scanFolders(mailbox: Mailbox): Promise<string[]> {
  const explicit = fileConfig.folders.scan != null;
  const discovered = fileConfig.folders.scan ?? (await mailbox.discoverFolders());
  return discovered.filter((f) => !isIgnored(f) && (explicit || !isConfidential(f)));
}

/** Collects unread mail from one account's mailbox as labelling rows (the training-data loop). */
async function exportRows(
  mailbox: Mailbox,
  account: ResolvedAccount,
): Promise<Array<Record<string, string>>> {
  const folders = await scanFolders(mailbox);
  log.info(`Folders: ${folders.join(', ')}`);
  const messages = await collectUnread(mailbox, folders);

  return messages.map((m) => ({
    account: account.id,
    folder: m.folder,
    fromName: m.fromName,
    fromAddress: m.fromAddress,
    subject: m.subject,
    receivedDateTime: m.receivedDateTime,
    internetMessageId: m.internetMessageId,
    decision: '',
    reason: '',
  }));
}

const ACTION_COLUMNS = ['Folder', 'Date', 'From', 'Subject', 'Action', 'Rule'] as const;

async function processMailbox(mailbox: Mailbox, cli: Cli): Promise<void> {
  const folders = await scanFolders(mailbox);
  log.info(`Folders: ${folders.join(', ')}`);

  const state: RunState = {
    cli,
    classifier: new RuleClassifier(rules),
    tally: { keep: 0, markRead: 0, delete: 0 },
    scanned: 0,
    actions: [],
  };

  for (const folder of folders) {
    if (cli.limit !== undefined && state.scanned >= cli.limit) {
      break;
    }
    try {
      await mailbox.process(folder, (session) => processFolder(session, state));
    } catch (error) {
      log.warn(`Skipping folder "${folder}": ${String(error)}`);
    }
  }

  const acted = [...state.actions].sort((a, b) => byNewestFirst(a.message, b.message));
  if (acted.length > 0) {
    const verb = cli.apply ? 'Applied' : 'Would apply';
    console.log(`\n### ${verb} — ${acted.length} action(s)\n`);
    const rows = acted.map((a) => [
      a.message.folder,
      dateCol(a.message),
      truncate(a.message.fromAddress, 32),
      truncate(a.message.subject || '(no subject)', 50),
      a.decision + (a.ok ? '' : ' (FAILED)'),
      a.reason,
    ]);
    console.log(markdownTable(ACTION_COLUMNS, rows));
    console.log();
  }

  log.info(
    `Summary — scanned: ${state.scanned}, keep: ${state.tally.keep}, ` +
      `markRead: ${state.tally.markRead}, delete: ${state.tally.delete}` +
      (cli.apply ? '' : ' (dry-run: nothing changed)'),
  );
}

/** Read-only inspection of one folder. Prints JSON to stdout; modifies nothing. */
async function inspect(mailbox: Mailbox, cli: Cli): Promise<void> {
  const folder = cli.folder ?? fileConfig.defaults.folder;
  if (isIgnored(folder)) {
    log.warn(`Folder "${folder}" is configured as ignore — not inspecting.`);
    return;
  }
  const messages = await mailbox.process(folder, (session) =>
    session.inspect({
      search: cli.search,
      unreadOnly: cli.unread,
      since: cli.since,
      limit: cli.limit,
    }),
  );
  log.info(`Inspect ${folder}: ${messages.length} message(s) (read-only, nothing changed)`);
  console.log(JSON.stringify(messages, null, 2));
}

/**
 * Prunes stale, already-read promotional mail from a folder (INBOX by default).
 * Only acts on READ messages, only deletes (never marks read here), and uses the
 * conservative cleanupRules + protect-list. Dry-run unless --apply.
 */
const CLEANUP_COLUMNS = ['Folder', 'Date', 'From', 'Subject', 'Rule', 'Expiry signal'] as const;

async function cleanup(mailbox: Mailbox, cli: Cli): Promise<void> {
  const folder = cli.folder ?? fileConfig.defaults.folder;
  if (isIgnored(folder)) {
    log.warn(`Folder "${folder}" is configured as ignore — not cleaning.`);
    return;
  }
  const classifier = new RuleClassifier(cleanupRules);

  await mailbox.process(folder, async (session) => {
    // Cleanup wants to sweep the whole window, not inspect's small default.
    const messages = await session.inspect({
      since: cli.since,
      limit: cli.limit ?? fileConfig.defaults.cleanupLimit,
    });

    // Only act on READ mail; collect matches, then sort newest-first for display.
    const matched = messages
      .filter((m) => m.isRead)
      .map((m) => ({ message: m, ...classifier.classify(m) }))
      .filter((r) => r.decision === 'delete')
      .sort((a, b) => byNewestFirst(a.message, b.message));
    const scanned = messages.filter((m) => m.isRead).length;

    const rows: string[][] = [];
    const byRule: Record<string, number> = {};
    let applied = 0;
    let failed = 0;

    for (const { message, reason } of matched) {
      byRule[reason] = (byRule[reason] ?? 0) + 1;
      rows.push([
        message.folder,
        dateCol(message),
        truncate(message.fromAddress, 32),
        truncate(message.subject || '(no subject)', 50),
        reason,
        expirySignal(message),
      ]);

      if (cli.apply) {
        try {
          await session.moveToDeleted(message.id);
          applied += 1;
        } catch (error) {
          failed += 1;
          log.error(`Failed to delete [${message.id}] "${message.subject}": ${String(error)}`);
        }
      }
    }

    if (rows.length > 0) {
      const verb = cli.apply ? 'Deleted (moved to Deleted Items)' : 'Would delete';
      console.log(`\n### ${verb} — ${rows.length} message(s) in ${folder}\n`);
      console.log(markdownTable(CLEANUP_COLUMNS, rows));
      console.log();
    }

    const breakdown = Object.entries(byRule)
      .map(([rule, count]) => `${rule}=${count}`)
      .join(', ');
    log.info(
      `Cleanup ${folder} — read scanned: ${scanned}, matched: ${rows.length}` +
        (breakdown ? ` (${breakdown})` : '') +
        (cli.apply ? `, deleted: ${applied}, failed: ${failed}` : ' (dry-run: nothing changed)'),
    );
  });
}

/** The accounts to process this run: all of them, or just the one named by --account. */
function selectAccounts(cli: Cli): ResolvedAccount[] {
  if (cli.account === undefined) {
    return accounts;
  }
  const id = cli.account.toLowerCase();
  const matched = accounts.filter((a) => a.id === id);
  if (matched.length === 0) {
    throw new Error(
      `No account with id "${cli.account}". Known: ${accounts.map((a) => a.id).join(', ')}.`,
    );
  }
  return matched;
}

/**
 * Authenticates an account into IMAP connection details, or returns null (logging a warning)
 * if its vendor's auth isn't implemented yet — so a run across accounts skips it and continues.
 */
async function connect(account: ResolvedAccount): Promise<MailboxConnection | null> {
  try {
    const creds = await authenticate(account);
    return {
      host: account.imapHost,
      port: account.imapPort,
      user: creds.user,
      accessToken: creds.accessToken,
      pass: creds.pass,
    };
  } catch (error) {
    if (error instanceof VendorNotImplementedError) {
      log.warn(`Skipping account "${account.id}" (${account.vendor}): ${error.message}.`);
    } else {
      log.error(`Skipping account "${account.id}" (${account.vendor}): ${String(error)}`);
    }
    return null;
  }
}

async function main(): Promise<void> {
  const cli = parseCli();
  const selected = selectAccounts(cli);

  // EXPORT aggregates unread mail from every account into one labelling CSV.
  if (cli.export !== undefined) {
    log.info('Mode: EXPORT');
    const allRows: Array<Record<string, string>> = [];
    for (const account of selected) {
      log.info(`\n── Account: ${account.id} (${account.vendor}) ──`);
      const conn = await connect(account);
      if (!conn) {
        continue;
      }
      await withMailbox(conn, async (mailbox) => {
        allRows.push(...(await exportRows(mailbox, account)));
      });
    }
    await writeFile(cli.export, toCsv(EXPORT_COLUMNS, allRows), 'utf8');
    log.info(
      `Exported ${allRows.length} unread message(s) across ${selected.length} account(s) to ${cli.export}.`,
    );
    log.info('Fill the "decision" (keep|markRead|delete) and "reason" columns, then save.');
    return;
  }

  const mode = cli.inspect
    ? 'INSPECT'
    : cli.cleanup
      ? `CLEANUP${cli.apply ? ' (APPLY)' : ' (DRY-RUN)'}`
      : `${cli.apply ? 'APPLY' : 'DRY-RUN'}${cli.limit ? ` (limit ${cli.limit})` : ''}`;
  log.info(`Mode: ${mode} — ${selected.length} account(s)`);

  for (const account of selected) {
    log.info(`\n══════ Account: ${account.id} (${account.vendor}) ══════`);
    const conn = await connect(account);
    if (!conn) {
      continue;
    }
    await withMailbox(conn, async (mailbox) => {
      if (cli.inspect) {
        return inspect(mailbox, cli);
      }
      if (cli.cleanup) {
        return cleanup(mailbox, cli);
      }
      return processMailbox(mailbox, cli);
    });
  }
}

main().catch((error: unknown) => {
  log.error(String(error instanceof Error ? error.stack ?? error.message : error));
  process.exitCode = 1;
});
