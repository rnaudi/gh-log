# gh-log

GitHub PR analytics for your terminal.

<img width="1548" height="2086" alt="carbon" src="https://github.com/user-attachments/assets/de17a0ea-c096-470e-8f23-692d99eecb2e" />


## Why?

Performance reviews. Need I say more?

Stop manually clicking through GitHub repos. This pulls your PR data in seconds, exports to JSON/CSV, and feeds nicely into Claude or ChatGPT to write the boring parts.

I track work via PRs with prefixes like "docs:", "review:", "meeting:". One `gh-log print --json` and I'm done.
   

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
./gh-log --version
```

Or download from [Latest release](https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-gnu.tar.gz)

## Quick Start

```bash
# View current month interactively
gh-log view

# Feed to an LLM for performance review
gh-log print --json | claude "Write 3 accomplishments from this data"
gh-log print | pbcopy  # then paste into ChatGPT

# Or just save it
gh-log print > review-$(date +%Y-%m).txt
```

## Usage

### Interactive TUI Mode
View PRs in an interactive terminal interface:

```bash
gh-log view --month 2025-01
gh-log view  # Defaults to current month
```

**Navigation:**
- `s` - Summary view (weekly and repo stats)
- `d` - Detail view (cycle between by week / by repository)
- `t` - Tail view (all PRs sorted by lead time)
- `↑↓` or `j/k` - Scroll
- `q` - Quit

### Print Mode
Print PR summary directly to terminal (includes PR descriptions):

```bash
gh-log print --month 2025-01
gh-log print  # Defaults to current month
```

**Output formats:**
```bash
gh-log print --month 2025-01        # Human-readable with descriptions
gh-log print --month 2025-01 --json # JSON format
gh-log print --month 2025-01 --csv  # CSV format
```

```csv
created_at,repo,number,title,lead_time_hours,size,additions,deletions,changed_files
2025-01-06T10:30:00Z,acme/api-gateway,1234,Add OAuth support,0.25,S,145,23,8
2025-01-07T14:15:00Z,acme/user-service,5678,Fix rate limiting bug,0.75,M,87,45,3
2025-01-08T09:20:00Z,acme/auth,9012,Refactor user service,0.20,S,42,18,5
...
```

### Doctor
```bash
gh-log doctor  # Check GitHub CLI and show cache/config locations
```

### Caching

Data is cached automatically (6h for current month, 24h for last month). Force refresh with `--force`.

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

## What You Get

Lead time, frequency, PR sizes (S/M/L/XL), top reviewers, review balance. Group by week or repository.