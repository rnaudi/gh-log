# gh-log

GitHub PR analytics for your terminal.

<img width="1548" height="2086" alt="carbon" src="https://github.com/user-attachments/assets/de17a0ea-c096-470e-8f23-692d99eecb2e" />


## Why?

**Performance review season.**  

Stop manually digging through GitHub repos. Get your PR data in seconds:
- Lead time, frequency, PR sizes (S/M/L/XL)
- Weekly breakdown, repo stats, top reviewers  
- Export to JSON/CSV

Feed it to Claude or ChatGPT, let the LLM write the boring parts.

*I track work via PRs with prefixes like "docs:", "review:", "meeting:". One `gh-log print --json` and I'm done.*
   

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

### Windows
Download from [Latest release](https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-pc-windows-msvc.zip)

Extract and run:
```powershell
gh-log.exe --version
```

## First Run

```bash
gh-log view
```



## Common Use Cases

**Performance reviews:**
```bash
gh-log print --json | claude "Summarize into 3 key accomplishments"
gh-log print | pbcopy  # paste into ChatGPT
```

**Export data:**
```bash
gh-log print --csv > prs-2026-01.csv
gh-log print > review.txt
```

**Different months:**
```bash
gh-log view --month 2025-12
gh-log print --month 2025-12 --force  # bypass cache
```

**Verify setup:**
```bash
gh-log doctor  # Check GitHub CLI, show cache/config paths
```

## Configuration (Optional)

```bash
gh-log config  # Shows location, creates template if missing
```

**Filter examples:**
```toml
[filter]
# Hide completely (not shown)
exclude_patterns = ["^test:", "^wip:", "^tmp:"]
exclude_repos = ["username/scratch"]

# Show but don't count in metrics  
ignore_patterns = ["^docs:", "^meeting:"]
ignore_repos = ["username/personal-notes"]

[size]
# Customize S/M/L/XL thresholds (lines changed)
small = 50
medium = 200  
large = 500
```

**Full documentation:** `gh-log config --help`  
Shows regex syntax, examples, and all options.