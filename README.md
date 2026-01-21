# gh-log

GitHub PR analytics for your terminal.

<img width="1079" height="698" alt="Screenshot 2026-01-21 at 17 55 46" src="https://github.com/user-attachments/assets/ce01387a-c6ea-48fb-9b1b-78111f97f184" />
<img width="1079" height="698" alt="Screenshot 2026-01-21 at 17 55 50" src="https://github.com/user-attachments/assets/72dfc228-cb97-441c-b8a5-3c522dacd9f8" />
<img width="1079" height="698" alt="Screenshot 2026-01-21 at 17 55 53" src="https://github.com/user-attachments/assets/2df1343e-565a-4526-810b-2dc4f760e9b5" />
<img width="1079" height="698" alt="Screenshot 2026-01-21 at 17 55 59" src="https://github.com/user-attachments/assets/334743ef-ca67-4a00-a61b-3c3e33718086" />



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
curl -L https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-musl.tar.gz | tar xz
./gh-log --version
```

Or download from [Latest release](https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-musl.tar.gz)

### Windows
```powershell
curl -L https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-pc-windows-msvc.zip -o gh-log.zip; tar -xf gh-log.zip
.\gh-log.exe --version
```

Or download from [Latest release](https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-pc-windows-msvc.zip)

**Verify the download (optional but recommended):**

Example for linux:

```bash
# Download the binary and checksum
curl -LO https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-musl.tar.gz
curl -LO https://github.com/rnaudi/gh-log/releases/latest/download/gh-log-x86_64-unknown-linux-musl.tar.gz.sha256

# Verify the checksum
sha256sum -c gh-log-x86_64-unknown-linux-musl.tar.gz.sha256

# If OK, extract
tar xzf gh-log-x86_64-unknown-linux-musl.tar.gz
./gh-log --version
```

## Shell Completion (Optional)

Tab completion is available for: **bash**, **zsh**, **fish**, **powershell**, **elvish**

```bash
gh-log completions --help  # Detailed instructions for all shells
```

Quick example for zsh:
```bash
gh-log completions zsh > ~/.zsh/completions/_gh-log
```

## Common Use Cases

**Interactive view:**
```bash
gh-log view  
```

Supports vim navigation: arrows, j/k (line), Ctrl-D/U (page), Ctrl-F/B (full page), g/G (top/bottom)

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
