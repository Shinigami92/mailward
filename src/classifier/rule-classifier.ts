import type { Classifier, Decision, MailMessage, Verdict } from '../types.js';

/**
 * A single ordered rule. The first rule whose `when` returns true wins, and its
 * `then` decision is applied. This is where logic that a webmail's built-in filter rules
 * can't express lives — arbitrary predicates combining sender, subject, body, age, etc.
 */
export interface Rule {
  /** Short identifier, surfaced as the decision reason. */
  name: string;
  when: (message: MailMessage) => boolean;
  then: Decision;
}

export class RuleClassifier implements Classifier {
  constructor(private readonly rules: readonly Rule[]) {}

  classify(message: MailMessage): Verdict {
    for (const rule of this.rules) {
      if (rule.when(message)) {
        return { decision: rule.then, reason: rule.name };
      }
    }
    return { decision: 'keep', reason: 'no rule matched' };
  }
}

/**
 * Helper: the leaf name of a (possibly nested) folder path, so rules can match
 * 'Archive/GitHub' as just 'GitHub'. Uses the IMAP '/' delimiter.
 */
export function folderName(message: MailMessage): string {
  return message.folder.split('/').pop() ?? message.folder;
}

/** Helper: whole-message age in hours, useful for "older than N" rules. */
export function ageInHours(message: MailMessage): number {
  const received = Date.parse(message.receivedDateTime);
  if (Number.isNaN(received)) {
    return 0;
  }
  return (Date.now() - received) / (1000 * 60 * 60);
}
