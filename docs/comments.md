# Comment taxonomy

Great comments are documentation. Every public item should carry a doc comment (`///` or `//!`) so rustdoc, IDE hovers, and `cargo doc` surface the same guidance reviewers rely on. Inline `//` belongs to private helpers or single statements that truly need annotation.

## Structuring doc comments

Reserve top-level Markdown headings (`# Overview`, `# Design`, `# Why`) for module or crate docs that benefit from a table-of-contents view.

For functions, structs, enums, and traits, lead with a concise summary and keep any rationale in plain prose. Keep `# Examples` for runnable snippets because rustdoc groups them automatically.

Module or crate files should start with `//!` comments that follow the same structure so readers can jump directly to features, file layout, or operational notes. When helpful, module docs can mix in headings such as:
- `# Overview` – summarize the intent and scope.
- `# Design` – call out trade-offs, invariants, and concurrency rules.
- `# Why` – explain non-obvious behaviors or ordering constraints.
- `# Examples` – provide runnable snippets or pseudo-code sketches.
- `# Limitations` or `# Follow-ups` – echo Debt or Checklist guidance when helpful.


Example:

```/dev/null/module.rs#L1-32
//! gh-log viewport renderer.
//!
//! # Overview
//! Renders summary, detail, and tail panes using ratatui. Keeps interaction
//! logic isolated so data and configuration layers stay testable.
//!
//! # Design
//! - Single `AppState` orchestrates the active view and scroll state.
//! - `DetailMode` toggles between weekly and repo breakdown without reallocations.
//! - Shared render buffer avoids flicker while switching panes.
//!
//! # Why
//! Scroll math tracks both content height and viewport height so resizes never
//! trap the cursor below the fold.
//!
//! # Examples
//! ```rust,no_run
//! # use gh_log::view::run_ui;
//! # fn main() -> anyhow::Result<()> {
//! let backend = ratatui::backend::CrosstermBackend::new(std::io::stdout());
//! let terminal = ratatui::Terminal::new(backend)?;
//! run_ui(terminal, month_data)?;
//! # Ok(())
//! # }
//! ```
```

## Function comments

- Explain what the function promises and when to call it so readers can skip the body.
- Keep the note beside the signature; the code and comment should travel together.
- Surface rationale in plain prose near the summary so the documentation reads naturally.
- Add a `# Examples` section when a snippet clarifies usage; rustdoc will render it in the standard Examples tab.
- Use `///`; for private helpers, fall back to `//` only if the guidance is truly local.

Example:

```gh-log/src/github.rs#L93-122
/// Fetch pull requests authored by the current user within the provided month (YYYY-MM).
///
/// GitHub search paginates reliably only with cursors, so we walk the entire page chain
/// to avoid missing PRs in busy months.
///
/// # Examples
/// ```rust,no_run
/// # use gh_log::github::CommandClient;
/// let client = CommandClient::new()?;
/// let prs = client.fetch_prs("2025-01")?;
/// println!("Fetched {} PRs", prs.len());
/// # anyhow::Ok::<_, anyhow::Error>(())
/// ```
pub fn fetch_prs(&self, month: &str) -> anyhow::Result<Vec<PullRequest>> {
    // ...
}
```

## Design comments

- Describe the big idea for the file, module, or subsystem and the trade-offs you accepted.
- Keep the rationale in clear sentences on public items so it lands in generated docs; reserve `# Design` headings for module roots documented with `//!`.
- Focus on invariants (“single writer, many readers”), concurrency or memory strategies, and what would break if the design changed.

Example:

```/dev/null/cache.rs#L1-24
//! Cache layer for month-level PR aggregates.
//!
//! # Design
//! - Write-through cache keyed by `YYYY-MM` keeps CLI invocations quick.
//! - File locking ensures the CLI and TUI never stomp on each other.
//! - JSON payloads stay stable so older binaries can read newer cache files.
```

## Why comments

- Spell out the hidden reason for an ordering, threshold, or guard clause.
- Work the reasoning directly into the doc comment; rely on inline `//` for a single statement that needs context.
- Highlight constraints (API quirks, time zones, data contracts) that aren’t obvious from the signature.

Example:

```gh-log/src/cache.rs#L88-99
/// Remove the stale cache file first so schema migrations never mix old and new
/// formats in a single document.
pub fn rewrite_cache(cache_file: &Path, data: &CachedData) -> Result<()> {
    fs::remove_file(cache_file)?;
    write_cache(cache_file, data)?;
    Ok(())
}
```

## Teacher comments

- Teach the background math, protocol, or data structure.
- Put the lesson immediately before the lines that rely on it.
- For reusable helpers, encode the explanation as a doc comment and include formulas or tables when they clarify behavior.

Example:

```gh-log/src/data.rs#L110-118
/// Lead time equals `updated_at - created_at`; clamp negatives to zero to hide clock skew.
pub fn lead_time(updated_at: DateTime<Utc>, created_at: DateTime<Utc>) -> Duration {
    (updated_at - created_at).max(Duration::zero())
}
```

## Checklist comments

- Remind maintainers about other spots to touch or the order to follow when tooling cannot enforce it.
- Keep reminders short and actionable; delete them once tests or automation cover the rule.
- Link to follow-up issues when the checklist implies ongoing work.

Example:

```gh-log/src/config.rs#L60-66
/// Adding another filter list requires updating `Config::validate()`
/// and the CLI regression tests in `tests/cli_tests.rs`.
pub struct FilterConfig {
    pub ignore_patterns: Vec<String>,
}
```

## Guide comments

- Break long functions into sections with light headings (Step 1/2/3).
- Use sparingly; if many steps are needed, consider extracting helpers.
- Inline `//` or block comments are acceptable here because the guidance is local to the routine.

Example:

```gh-log/src/view.rs#L210-222
// Step 1: compute panel rectangles before drawing widgets.
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(summary_height), Constraint::Min(0)])
    .split(area);

// Step 2: render summary panel first so scrollbar state stays coherent.
render_summary(frame, chunks[0], month);
```

## Trivial comments

- Delete comments that merely restate the code.
- Replace them with Function, Why, or Teacher guidance if deeper context is needed.

## Debt comments

- Mark shortcuts with clear exit criteria and a review date.
- Prefer `TODO(issue#)` or `FIXME` with a link; escalate long-lived notes into the issue tracker.
- Revisit debt comments during each release cut.

Example:

```gh-log/src/cache.rs#L150-156
/* TODO:#182 cap cache JSON at 5 MB once serializer benchmarks land. */
if cache_size > MAX_CACHE_BYTES {
    schedule_compaction();
}
```

## Backup comments

- Never check in old code blocks commented out. Version control already stores history.
- Delete temporary fallbacks once the new behavior ships.

## Review checklist

- [ ] Doc comments start with a concise summary; module docs use headings while item docs keep rationale in flowing prose.
- [ ] Public APIs include `# Examples` when usage benefits from a snippet.
- [ ] Design and Why rationales appear where the next maintainer will look first.
- [ ] Checklist and Debt notes point to enforceable steps or tracked issues.
- [ ] Trivial or stale comments were culled during the change.

References:

- [“Writing system software: code comments”](https://www.antirez.com/news/124)
- i[Bitask doc comment example](https://github.com/vrnvu/bitask/blob/master/src/db.rs)
