/** A single inbox message, as projected from the Microsoft Graph `messages` resource. */
export interface MailMessage {
  id: string;
  /** IMAP folder the message lives in, e.g. 'INBOX', 'Junk', 'GitHub'. */
  folder: string;
  subject: string;
  /** Sender email address, lower-cased, or '' if Graph omitted it. */
  fromAddress: string;
  /** Sender display name, or '' if Graph omitted it. */
  fromName: string;
  /** ISO 8601 timestamp the message was received. */
  receivedDateTime: string;
  /** First ~255 chars of the body, plain text. */
  bodyPreview: string;
  isRead: boolean;
  hasAttachments: boolean;
  importance: 'low' | 'normal' | 'high';
  internetMessageId: string;
}

/** What the helper decides to do with a message. */
export type Decision = 'keep' | 'markRead' | 'delete';

export interface Verdict {
  decision: Decision;
  /** Human-readable explanation, e.g. the name of the rule that matched. */
  reason: string;
}

/**
 * Pluggable decision engine. The PoC ships a rule-based implementation; an
 * AI-backed one (Claude API) can be dropped in later without touching the pipeline.
 */
export interface Classifier {
  classify(message: MailMessage): Verdict;
}
