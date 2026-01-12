# gh-log

GitHub PR analytics for your terminal.

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
