//! Behavioral parity harness for the ported rule engine.
//!
//! It compiles the shipped `rules.example.yaml` and runs a fixture message
//! through both the `classify` and `cleanup` rule sets, snapshotting every
//! decision. The snapshot is the parity record against the v1 TypeScript engine
//! - review it when rules change.

use insta::assert_debug_snapshot;
use mailward_core::{MailMessage, RulesDocument, Verdict, first_match};

/// The shipped example rules double as the parity fixture's rule source.
const RULES_YAML: &str = include_str!("../../../rules.example.yaml");

fn msg(
    folder: &str,
    subject: &str,
    from_address: &str,
    from_name: &str,
    body_preview: &str,
    age_hours: f64,
) -> MailMessage {
    MailMessage {
        id: "test".to_string(),
        folder: folder.to_string(),
        subject: subject.to_string(),
        from_address: from_address.to_string(),
        from_name: from_name.to_string(),
        body_preview: body_preview.to_string(),
        age_hours,
        is_read: true,
    }
}

#[derive(Debug)]
#[allow(dead_code)] // fields are read via the Debug snapshot
struct Outcome {
    label: &'static str,
    classify: Verdict,
    cleanup: Verdict,
}

#[test]
fn rule_decisions_snapshot() {
    let doc = RulesDocument::from_yaml_str(RULES_YAML).expect("example rules parse");
    let rules = doc.compile_for(None).expect("example rules compile");

    // (label, message) - spans every classify and cleanup rule plus keep cases.
    let fixtures: Vec<(&'static str, MailMessage)> = vec![
        // --- classify ---
        (
            "github-bot",
            msg(
                "Archive/GitHub",
                "PR opened",
                "noreply@github.com",
                "dependabot[bot]",
                "",
                1.0,
            ),
        ),
        (
            "github-human-kept",
            msg(
                "Archive/GitHub",
                "PR opened",
                "jane@github.com",
                "Jane Dev",
                "",
                1.0,
            ),
        ),
        (
            "known-newsletter",
            msg(
                "INBOX",
                "Weekly news",
                "hello@newsletter.example.com",
                "News",
                "",
                1.0,
            ),
        ),
        (
            "junk-spam-domain",
            msg("Junk", "hi", "x@spam-host.example", "X", "", 1.0),
        ),
        (
            "junk-spam-onmicrosoft",
            msg("Junk", "hi", "x@random.onmicrosoft.com", "X", "", 1.0),
        ),
        (
            "junk-spam-subject",
            msg(
                "Junk",
                "Win a CASINO jackpot now",
                "x@unknown.test",
                "X",
                "",
                1.0,
            ),
        ),
        (
            "junk-phishing",
            msg(
                "Junk",
                "Notice",
                "x@unknown.test",
                "X",
                "Please verify your account immediately",
                1.0,
            ),
        ),
        (
            "junk-money-scam",
            msg(
                "Junk",
                "Business proposal",
                "x@unknown.test",
                "X",
                "I have millions to share",
                1.0,
            ),
        ),
        (
            "junk-emotional-scam",
            msg(
                "Junk",
                "Greetings",
                "x@unknown.test",
                "X",
                "you are my next of kin, a blessing",
                1.0,
            ),
        ),
        (
            "junk-impersonation",
            msg(
                "Junk",
                "Your iCloud was locked",
                "x@unknown.test",
                "X",
                "",
                1.0,
            ),
        ),
        (
            "junk-fake-parcel",
            msg(
                "Junk",
                "Your parcel is out for delivery",
                "x@unknown.test",
                "X",
                "",
                1.0,
            ),
        ),
        (
            "junk-unmatched-kept",
            msg(
                "Junk",
                "Lunch tomorrow?",
                "friend@unknown.test",
                "Friend",
                "hi",
                1.0,
            ),
        ),
        (
            "inbox-unmatched-kept",
            msg(
                "INBOX",
                "Hello from a friend",
                "friend@unknown.test",
                "Friend",
                "hi",
                1.0,
            ),
        ),
        // --- cleanup ---
        (
            "protect-sender",
            msg(
                "INBOX",
                "Mega sale last chance",
                "billing@your-bank.example",
                "Bank",
                "",
                999.0,
            ),
        ),
        (
            "protect-subject",
            msg(
                "INBOX",
                "Your invoice #123",
                "x@deals.example.com",
                "Deals",
                "",
                999.0,
            ),
        ),
        (
            "monthly-stale-old",
            msg(
                "INBOX",
                "Digest",
                "noreply@digest.example.com",
                "Digest",
                "",
                800.0,
            ),
        ),
        (
            "monthly-stale-young",
            msg(
                "INBOX",
                "Digest",
                "noreply@digest.example.com",
                "Digest",
                "",
                100.0,
            ),
        ),
        (
            "promo-digest-old",
            msg(
                "INBOX",
                "Weekly update",
                "noreply@deals.example.com",
                "Deals",
                "",
                800.0,
            ),
        ),
        (
            "promo-digest-young",
            msg(
                "INBOX",
                "Weekly update",
                "noreply@deals.example.com",
                "Deals",
                "",
                100.0,
            ),
        ),
        (
            "promo-expired",
            msg(
                "INBOX",
                "Mega sale",
                "noreply@deals.example.com",
                "Deals",
                "Last chance to buy!",
                48.0,
            ),
        ),
        (
            "promo-stale",
            msg(
                "INBOX",
                "Mega sale",
                "noreply@deals.example.com",
                "Deals",
                "buy now",
                300.0,
            ),
        ),
        (
            "promo-young-kept",
            msg(
                "INBOX",
                "Mega sale",
                "noreply@deals.example.com",
                "Deals",
                "buy now",
                10.0,
            ),
        ),
    ];

    let outcomes: Vec<Outcome> = fixtures
        .iter()
        .map(|(label, message)| Outcome {
            label,
            classify: first_match(&rules.classify, message),
            cleanup: first_match(&rules.cleanup, message),
        })
        .collect();

    assert_debug_snapshot!(outcomes);
}

#[test]
fn per_account_override_replaces_a_list() {
    let yaml = r#"
refs:
  lists:
    markReadDomains: [base.example]
  patterns: {}
  thresholds: {}
classify:
  - name: known-newsletter
    when: { field: fromAddress, op: endsWithAny, ref: lists.markReadDomains }
    then: markRead
cleanup: []
accounts:
  work:
    refs:
      lists:
        markReadDomains: [work.example]
"#;
    let doc = RulesDocument::from_yaml_str(yaml).unwrap();

    let base = doc.compile_for(None).unwrap();
    let work = doc.compile_for(Some("work")).unwrap();

    let from_base = msg("INBOX", "hi", "a@base.example", "A", "", 1.0);
    let from_work = msg("INBOX", "hi", "a@work.example", "A", "", 1.0);

    use mailward_core::Decision;
    // Base account marks base.example read; work account no longer does.
    assert_eq!(
        first_match(&base.classify, &from_base).decision,
        Decision::MarkRead
    );
    assert_eq!(
        first_match(&work.classify, &from_base).decision,
        Decision::Keep
    );
    // Work account marks its own override domain read.
    assert_eq!(
        first_match(&work.classify, &from_work).decision,
        Decision::MarkRead
    );
}
