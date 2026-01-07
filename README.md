# gh-log

View your GitHub PRs summary in a TUI or print to terminal.

Do **not** use this tool to automatically generate performance review reports or similar with ChatGPT.

## Installation

### Homebrew
```bash
brew install rnaudi/tap/gh-log
```

### Releases
See [Releases](https://github.com/rnaudi/gh-log/releases)

## What it does

- Calculates lead time, frequency, and other metrics
- Groups by month, week, and repository
- Two modes: interactive TUI or terminal print

## Usage

### Interactive TUI Mode
View PRs in an interactive terminal interface:

```bash
gh-log view --month 2025-01
```

Navigate views:
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

## TODO: Advanced / Use cases

### Pull request tracking
- Track PRs across repositories
- Calculate DORA metrics: deployment frequency and lead time
- Group by month, week, and repository
- Useful for work logs and performance reviews

### Code review tracking
- Create PRs in a private repo with title pattern: `Review: owner/repo#123`
- Track lead time from approval to merge
- Filter by title pattern to separate reviews from regular work

### Documentation tracking
- Use PRs for documentation work
- Track lead time from creation to merge
- Shows up in work log metrics

### Conventional commits
- Filter PRs by title patterns: `fix:`, `feat:`, `refactor:`, `docs:`, etc.
- Track lead time for bug fixes vs features
- Compare frequency and velocity by commit type
- Analyze maintenance vs feature work distribution

### Contribution distribution
- See which repositories get most attention
- Track cross-repo contribution patterns
- Identify repos with longer lead times

### Performance review / brag document generation
- Export PR data and metrics for LLM-based brag document generation
- Use PR titles, lead times, and frequency to create accomplishment summaries
- Track work across repos and time periods for performance reviews
- Generate structured data for documenting projects and contributions
