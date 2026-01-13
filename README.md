# gh-log

GitHub PR analytics for your terminal.

<img width="1548" height="2086" alt="carbon" src="https://github.com/user-attachments/assets/de17a0ea-c096-470e-8f23-692d99eecb2e" />


## Why?

Performance review season rolls around and suddenly you're manually digging through GitHub trying to answer "what did I even do this month?" Clicking through repos, counting PRs, calculating averages, tedious work that I'd rather not spend brain cycles on.

This pulls the data in seconds. Export to JSON, feed it to Claude or ChatGPT, and let the LLM write the boring parts of your review. Or dump it in a spreadsheet if that's your thing.

I keep a private personal repository where I open PRs with prefixes like "docs:", "review:", "meeting:". PRs become the source of truth for most of my work, and the only cost is opening and merging a PR with a descriptive title that an LLM can interpret.
   

## Prerequisites

Requires [GitHub CLI](https://cli.github.com/) (`gh`) to be installed and authenticated.

## Installation

### macOS
```bash
brew install rnaudi/tap/gh-log
gh-log --version
```

Or download from [Latest release](https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-aarch64-apple-darwin.tar.gz)

### Linux / Unix
```bash
curl -L https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-gnu.tar.gz | tar xz
chmod +x gh-log
./gh-log --version
```

Or download from [Latest release](https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-gnu.tar.gz)

## Usage

### Interactive TUI Mode
View PRs in an interactive terminal interface:

```bash
gh-log view --month 2025-01
```

**Navigation:**
- `s` - Summary view (weekly and repo stats)
- `d` - Detail view (PRs grouped by week)
- `t` - Tail view (all PRs sorted by lead time)
- `↑↓` or `j/k` - Scroll
- `q` - Quit

### Print Mode
Print PR summary directly to terminal:

```bash
gh-log print --month 2025-01
```

**Output formats:**
```bash
gh-log print --month 2025-01
gh-log print --month 2025-01 --json
gh-log print --month 2025-01 --csv
```

~~~
created_at,repo,number,title,lead_time_hours,size,additions,deletions,changed_files
2025-01-06T10:30:00Z,acme/api-gateway,1234,Add OAuth support,0.25,S,145,23,8
2025-01-07T14:15:00Z,acme/user-service,5678,Fix rate limiting bug,0.75,M,87,45,3
2025-01-08T09:20:00Z,acme/auth,9012,Refactor user service,0.20,S,42,18,5
2025-01-09T11:45:00Z,acme/api-gateway,1235,Update dependencies,0.13,S,312,287,12
2025-01-10T15:30:00Z,acme/api-gateway,1236,Add metrics endpoint,0.30,S,68,12,4
2025-01-11T08:00:00Z,acme/user-service,5679,Improve error handling,0.53,S,95,34,6
2025-01-11T16:45:00Z,acme/auth,9013,Add 2FA support,1.00,L,234,67,15
2025-01-11T18:30:00Z,acme/api-gateway,1237,Fix memory leak,0.37,S,45,28,2
2025-01-13T10:00:00Z,acme/api-gateway,1238,Optimize database queries,0.47,S,78,56,4
2025-01-13T14:30:00Z,acme/auth,9014,Add session management,0.92,M,156,89,9
2025-01-14T09:15:00Z,acme/user-service,5680,Add user profile API,1.00,L,289,123,18
2025-01-15T11:00:00Z,acme/api-gateway,1239,Update API docs,0.17,S,234,156,1
2025-01-16T13:20:00Z,acme/api-gateway,1240,Add rate limit headers,0.25,S,34,12,2
2025-01-17T10:45:00Z,acme/user-service,5681,Fix validation bug,0.70,S,67,45,3
2025-01-17T15:00:00Z,acme/auth,9015,Refactor auth middleware,0.58,S,98,76,7
2025-01-18T09:30:00Z,acme/api-gateway,1241,Add health check endpoint,0.20,S,28,8,2
2025-01-18T11:15:00Z,acme/api-gateway,1242,Update logging,0.13,S,45,23,3
2025-01-18T14:00:00Z,acme/user-service,5682,Add pagination support,0.80,M,145,89,11
2025-01-18T16:45:00Z,acme/auth,9016,Fix token refresh,0.42,S,56,34,4
2025-01-18T18:20:00Z,acme/api-gateway,1243,Improve error messages,0.30,S,67,45,5
~~~

### Doctor Command
Check system requirements and diagnostics:

~~~bash
gh-log doctor
~~~

Output:
~~~bash
gh-log diagnostics

✓ GitHub CLI: gh version 2.83.2 (2025-12-10)
https://github.com/cli/cli/releases/tag/v2.83.2

Cache directory: /Users/user/Library/Caches/gh-log
  2025-01.json (2026-01-12 13:22:08 UTC)
  2025-11.json (2026-01-12 16:59:35 UTC)
  2025-12.json (2026-01-12 16:59:16 UTC)
  2026-01.json (2026-01-12 16:42:19 UTC)

Configuration file: /Users/user/Library/Application Support/gh-log/config.toml
  (exists)
~~~

Shows:
- GitHub CLI installation status
- Cache directory location and files
- Configuration file location

### Caching

Data is automatically cached for faster subsequent requests.

**Cache TTL:**
- Current month: 6 hours
- Last month: 24 hours
- Older months: never expires

**Force refresh:**
```bash
gh-log view --month 2025-01 --force
gh-log print --month 2025-01 --force
```

**Cache location:**
- macOS: `~/Library/Caches/gh-log/`
- Linux: `~/.cache/gh-log/`
- Windows: `%LOCALAPPDATA%\gh-log\cache\`

## Configuration

Run `gh-log config` to create a config file, then edit it:

```toml
[filter]
# Exclude = hidden completely (not shown)
exclude_repos = ["username/spam-repo"]
exclude_patterns = ["^test:", "^tmp:", "^wip:"]

# Ignore = shown but not counted in metrics
ignore_repos = ["username/personal-notes", "username/scratch"]
ignore_patterns = ["^docs:", "^meeting:", "^review:"]

[size]
# Customize S/M/L/XL thresholds (lines changed)
small = 50
medium = 200
large = 500
```

**What this does:**

- PRs from `spam-repo` won't appear anywhere
- PRs titled "test: something" or "tmp: debug" won't appear
- PRs from `personal-notes` appear but don't count in metrics
- PRs titled "docs: update readme" appear but don't count in metrics
- Custom sizes: S ≤50, M 51-200, L 201-500, XL >500

Patterns use regex syntax. If a repo is both excluded and ignored, it gets excluded.

## Metrics

- **Lead Time**: Time from PR creation to merge
- **Frequency**: PRs per week
- **PR Size**: Categorized as S/M/L/XL based on lines changed and files modified
- **Size Distribution**: Count of S/M/L/XL PRs per repository and overall
- **Top Reviewers**: Who reviewed your PRs most
- **Review Activity**: PRs you reviewed for others
- **Review Balance**: Ratio of PRs reviewed vs created
- **Weekly Breakdown**: PRs grouped by week with aggregated metrics
- **Repository Breakdown**: PR activity grouped by repository
