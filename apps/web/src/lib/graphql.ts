import { gql } from "@urql/vue";

export const ACCOUNTS_QUERY = gql`
  query Accounts {
    accounts {
      id
      vendor
      username
      imapHost
      authMethod
      authenticated
    }
  }
`;

export const CONFIG_YAML_QUERY = gql`
  query ConfigYaml {
    configYaml
  }
`;

export const RULES_YAML_QUERY = gql`
  query RulesYaml {
    rulesYaml
  }
`;

export const ACCOUNTS_YAML_QUERY = gql`
  query AccountsYaml {
    accountsYaml
  }
`;

export const UPDATE_CONFIG_YAML = gql`
  mutation UpdateConfigYaml($content: String!) {
    updateConfigYaml(content: $content)
  }
`;

export const UPDATE_RULES_YAML = gql`
  mutation UpdateRulesYaml($content: String!) {
    updateRulesYaml(content: $content)
  }
`;

export const UPDATE_ACCOUNTS_YAML = gql`
  mutation UpdateAccountsYaml($content: String!) {
    updateAccountsYaml(content: $content)
  }
`;

export const AUTH_START = gql`
  mutation AuthStart($account: String!) {
    authStart(account: $account) {
      authorizeUrl
    }
  }
`;

export const AUTH_COMPLETE = gql`
  mutation AuthComplete($account: String!, $redirectUrl: String!) {
    authComplete(account: $account, redirectUrl: $redirectUrl)
  }
`;

export const TRIGGER_RUN = gql`
  mutation TriggerRun($account: String, $mode: RunMode!, $dryRun: Boolean!, $runId: String) {
    triggerRun(account: $account, mode: $mode, dryRun: $dryRun, runId: $runId) {
      runId
      scanned
      actioned
      applied
      decisions {
        account
        folder
        uid
        subject
        from
        fromName
        decision
        reason
      }
    }
  }
`;

export const INSPECTABLE_FOLDERS = gql`
  query InspectableFolders($account: String!) {
    inspectableFolders(account: $account)
  }
`;

export const INSPECT = gql`
  query Inspect($account: String!, $folder: String!, $limit: Int!, $unreadOnly: Boolean!) {
    inspect(account: $account, folder: $folder, limit: $limit, unreadOnly: $unreadOnly) {
      uid
      folder
      subject
      from
      fromName
      isRead
      ageHours
      bodyPreview
    }
  }
`;

export const RUN_PROGRESS = gql`
  subscription RunProgress {
    runProgress {
      runId
      kind
      folder
      index
      total
      fetched
      fetchTotal
      decision {
        account
        folder
        uid
        subject
        from
        fromName
        decision
        reason
      }
    }
  }
`;
