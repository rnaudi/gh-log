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

**Output formats:**
- Default (raw): Human-readable summary format
- JSON: Machine-readable JSON format
- CSV: CSV format for spreadsheet analysis

```bash
gh-log print --month 2025-01

gh-log print --month 2025-01 --json

gh-log print --month 2025-01 --csv
```

### Caching

Data is automatically cached for faster subsequent requests.

**Cache behavior:**
- First request: fetches from GitHub (2-3 seconds)
- Subsequent requests: loads from cache (instant!)
- Cache TTL: Current month (6h), Last month (24h), Older months (never expires)

**Force refresh:**
```bash
gh-log view --month 2025-01 --force   # Bypass cache
gh-log print --month 2025-01 --force  # Bypass cache
```

Cache is stored in platform-specific directories:
- macOS: `~/Library/Caches/gh-log/`
- Linux: `~/.cache/gh-log/`
- Windows: `%LOCALAPPDATA%\gh-log\cache\`

## Features

### Current Metrics
- **Lead Time**: Time from PR creation to merge
- **Frequency**: PRs per week
- **PR Size**: Categorized as S/M/L/XL based on lines changed and files modified
- **Size Distribution**: Count of S/M/L/XL PRs per repository and overall
- **Weekly Breakdown**: PRs grouped by week with aggregated metrics
- **Repository Breakdown**: PR activity grouped by repository

### Current Views
- **Summary View**: Weekly and repository statistics
- **Detail View**: PRs grouped by week

## TODO: Enhancements

### DORA Metrics

#### 1. Lead Time Breakdown (High Priority)
Break down lead time into stages to identify bottlenecks:
- Time to First Review (creation → first review)
- Time in Review (first review → approval)
- Time to Merge (approval → merge)
- Show which stage is the bottleneck

**Example Output:**
```
Lead Time Breakdown:
  - Creation → First Review: 2h (bottleneck!)
  - First Review → Approval: 30m
  - Approval → Merge: 15m
  - Total: 2h 45m
```

#### 2. Change Failure Rate
Track PRs that led to failures or needed fixes:
- Identify PRs with patterns: `revert:`, `hotfix:`, `fix:`
- Calculate % of PRs that needed follow-up fixes
- Track time to fix (Time to Restore Service)

#### 3. Deployment Frequency Enhancement
Filter PRs by target branch:
- Track only production deploys (PRs to `main`, `production`, etc.)
- Separate feature work from actual deployments
- Compare deployment frequency across repos

### Review & Collaboration Metrics

#### 4. Top Reviewers (High Priority)
Track who reviews your PRs:
- List reviewers with PR count
- Show average approval time per reviewer
- Identify key collaborators and responsive reviewers

**GraphQL Fields:**
```graphql
reviews {
  nodes {
    author { login }
    state
    submittedAt
  }
}
latestReviews {
  nodes {
    author { login }
  }
}
```

**Example Output:**
```
Top Reviewers:
  - alice: 12 PRs (avg approval: 1h)
  - bob: 8 PRs (avg approval: 30m)
  - charlie: 5 PRs (avg approval: 3h)
```

#### 5. My Review Activity (High Priority)
Track PRs you've reviewed for others:
- Show repos you review most
- Show authors you help most
- Calculate review balance ratio (PRs created vs reviewed)

**GraphQL Query:**
```graphql
search(query: "is:pr reviewed-by:@me created:YYYY-MM")
```

**Example Output:**
```
My Review Activity:
  - PRs Reviewed: 45
  - Repos: scopely/heimdall (20), scopely/auth (15)
  - Top Authors: alice (15 PRs), bob (12 PRs)
  - Review Balance: 1.3:1 (45 reviewed / 34 created)
```

#### 6. Collaboration Patterns
Track PR collaboration metrics:
- Average participants per PR
- Number of comments per PR
- Most collaborative repos (high participant count)
- Identify solo work vs team work patterns

**GraphQL Fields:**
```graphql
comments { totalCount }
participants {
  totalCount
  nodes { login }
}
```

### PR Health & Quality Metrics

#### 7. Average Lead Time by Size (High Priority)
Show correlation between PR size and lead time:
- Calculate average lead time for each size category (S/M/L/XL)
- Prove that smaller PRs merge faster
- Actionable insight: "Keep PRs small!"

**Example Output:**
```
Lead Time by Size:
  - S: 15m avg (26 PRs)
  - M: 1h 30m avg (3 PRs)
  - L: 4h avg (4 PRs)
  - XL: 12h avg (1 PR)
```

#### 8. PR Health Indicators
Track PR quality signals:
- Average commits per PR
- Average comments per PR
- PRs with changes requested (%)
- Draft PR usage
- Review decision stats (approved/changes requested/review required)

**GraphQL Fields:**
```graphql
commits { totalCount }
comments { totalCount }
reviewDecision
isDraft
```

#### 9. Percentiles for Lead Time
Show distribution beyond just average:
- p50 (median)
- p90
- p95
- Identify outliers better than average alone

**Example Output:**
```
Lead Time Distribution:
  - Avg: 35m
  - p50: 12m (half merge within 12m)
  - p90: 1h 30m
  - p95: 2h 15m
```

### Workflow & Analysis

#### 10. Week-Level Size Distribution
Track how PR sizes change over time:
- Show size breakdown per week
- Identify trends (getting bigger/smaller?)
- Spot unusual weeks

**Example Output:**
```
Week 1 (2026-01-05 - 2026-01-11)
  - PRs: 34 [26S 3M 4L 1XL]
  - Avg Lead Time: 35m
```

#### 11. Day-of-Week Analysis
Understand work patterns:
- PRs created by day of week
- PRs merged by day of week
- Identify "no PR Fridays" or "Monday pile-up"

#### 12. Export to CSV/JSON
Enable further analysis:
```bash
gh-log print --month 2026-01 --csv > prs.csv
gh-log print --month 2026-01 --json > prs.json
```

#### 13. Filter by Repository
In TUI mode, filter to specific repo:
- Press `f` to filter
- Useful for multi-repo workflows
- Focus on specific project metrics

#### 14. Conventional Commits Support
Filter and categorize by commit type:
- `fix:`, `feat:`, `refactor:`, `docs:`, `chore:`, etc.
- Track lead time by commit type
- Compare frequency (features vs fixes)
- Analyze maintenance vs feature work ratio

### Advanced Use Cases

#### Code Review Tracking
- Create PRs in a private repo with pattern: `Review: owner/repo#123`
- Track lead time for review work
- Separate personal PRs from review contributions
- Show up in review activity metrics

#### Documentation Tracking
- Use PRs for documentation work
- Track docs lead time separately
- Show documentation contributions in metrics

#### Multi-Month Trends
- Compare metrics across multiple months
- Show month-over-month changes
- Identify improving/declining trends
- Track team velocity over time

#### Performance Review / Brag Document Generation
- Export PR data for LLM-based brag document generation
- Use PR titles, lead times, and metrics for accomplishment summaries
- Track work across repos and time periods
- Generate structured data for performance reviews
