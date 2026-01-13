# gh-log

GitHub PR analytics for your terminal.

~~~
┌─ GitHub PRs for 2025-01 ──────────────────────────────────────────┐
│ Total PRs: 34 │ Avg Lead Time: 35m │ Frequency: 8.5/week          │
│ Sizes: [26S 3M 4L 1XL] │ Review Balance: 1.3:1 (45 reviewed)      │
└────────────────────────────────────────────────────────────────────┘

━━━ Week 1 (Jan 05 - Jan 11) │ 8 PRs │ Avg: 25m ━━━━━━━━━━━━━━━━━━━

  Jan 06 │ acme/api-gateway   │ #1234 Add OAuth support          │ 15m │ S
  Jan 07 │ acme/user-service  │ #5678 Fix rate limiting bug      │ 45m │ M
  Jan 08 │ acme/auth          │ #9012 Refactor user service      │ 12m │ S
  Jan 09 │ acme/api-gateway   │ #1235 Update dependencies        │  8m │ S
  Jan 10 │ acme/api-gateway   │ #1236 Add metrics endpoint       │ 18m │ S
  Jan 11 │ acme/user-service  │ #5679 Improve error handling     │ 32m │ S
  Jan 11 │ acme/auth          │ #9013 Add 2FA support            │ 1h  │ L
  Jan 11 │ acme/api-gateway   │ #1237 Fix memory leak            │ 22m │ S

━━━ Week 2 (Jan 12 - Jan 18) │ 12 PRs │ Avg: 38m ━━━━━━━━━━━━━━━━━━

  Jan 13 │ acme/api-gateway   │ #1238 Optimize database queries  │ 28m │ S
  Jan 13 │ acme/auth          │ #9014 Add session management     │ 55m │ M
  Jan 14 │ acme/user-service  │ #5680 Add user profile API       │ 1h  │ L
  Jan 15 │ acme/api-gateway   │ #1239 Update API docs            │ 10m │ S
  Jan 16 │ acme/api-gateway   │ #1240 Add rate limit headers     │ 15m │ S
  Jan 17 │ acme/user-service  │ #5681 Fix validation bug         │ 42m │ S
  Jan 17 │ acme/auth          │ #9015 Refactor auth middleware   │ 35m │ S
  Jan 18 │ acme/api-gateway   │ #1241 Add health check endpoint  │ 12m │ S
  Jan 18 │ acme/api-gateway   │ #1242 Update logging             │  8m │ S
  Jan 18 │ acme/user-service  │ #5682 Add pagination support     │ 48m │ M
  Jan 18 │ acme/auth          │ #9016 Fix token refresh          │ 25m │ S
  Jan 18 │ acme/api-gateway   │ #1243 Improve error messages     │ 18m │ S

━━━ Week 3 (Jan 19 - Jan 25) │ 9 PRs │ Avg: 42m ━━━━━━━━━━━━━━━━━━━

  Jan 20 │ acme/user-service  │ #5683 Add search functionality   │ 1h  │ L
  Jan 21 │ acme/api-gateway   │ #1244 Fix CORS configuration     │ 38m │ S
  Jan 22 │ acme/auth          │ #9017 Add password reset         │ 52m │ M
  Jan 23 │ acme/api-gateway   │ #1245 Refactor request handlers  │ 35m │ S
  Jan 24 │ acme/user-service  │ #5684 Fix user deletion bug      │ 28m │ S
  Jan 25 │ acme/api-gateway   │ #1246 Add request validation     │ 45m │ S
  Jan 25 │ acme/auth          │ #9018 Update security headers    │ 18m │ S
  Jan 25 │ acme/api-gateway   │ #1247 Improve cache strategy     │ 1h  │ L
  Jan 25 │ acme/user-service  │ #5685 Add email verification     │ 38m │ S

┌─ Controls ────────────────────────────────────────────────────────┐
│ s: Summary │ d: Details │ t: Tail │ ↑↓/jk: Scroll │ q: Quit       │
└────────────────────────────────────────────────────────────────────┘
~~~

## Why?

Performance review season rolls around and suddenly you're manually digging through GitHub trying to answer "what did I even do this month?" Clicking through repos, counting PRs, calculating averages, tedious work that I'd rather not spend brain cycles on.

This pulls the data in seconds. Export to JSON, feed it to Claude or ChatGPT, and let the LLM write the boring parts of your review. Or dump it in a spreadsheet if that's your thing.

I keep a private personal repository where I open PRs with prefixes like "docs:", "review:", "meeting:". PRs become the source of truth for most of my work, and the only cost is opening and merging a PR with a descriptive title that an LLM can interpret.
   

## Prerequisites

Requires [GitHub CLI](https://cli.github.com/) (`gh`) to be installed and authenticated.

## Installation

### Homebrew
```bash
brew install rnaudi/tap/gh-log
```

### Releases
See [Releases](https://github.com/rnaudi/gh-log/releases)

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
