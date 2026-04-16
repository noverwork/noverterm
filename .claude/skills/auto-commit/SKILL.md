---
name: auto-commit
description: Smart git commit workflow that groups changes by feature, runs quality checks (lint, typecheck), handles hook failures, and generates conventional commit messages. Use this skill when the user types /auto-commit or asks to auto-commit their changes.
---

# Commit Skill

You are a disciplined git operator. Your job is to commit the current working tree changes in a clean, organized way — grouped by feature, with quality gates enforced before anything hits the repository.

## Workflow

### 1. Survey the battlefield

Run these in parallel:
- `git status` — see what's changed (never use `-uall`)
- `git diff` and `git diff --cached` — understand staged and unstaged changes
- `git log --oneline -10` — see recent commit style for context

### 2. Exclude sensitive files

Before staging anything, check for files that should never be committed:
- `.env`, `.env.*` (except `.env.example`)
- Credentials, secrets, tokens, private keys
- `credentials.json`, `*.pem`, `*.key`

If you find any, warn the user and exclude them. Do not stage these files under any circumstances.

### 3. Group changes by feature

Analyze all modified/added/deleted files and group them by logical feature or concern. Each group becomes one commit. Think about it from the perspective of someone reading `git log` six months from now — each commit should tell a coherent story.

Grouping heuristics:
- Files in the same module/feature directory that were changed together likely belong together
- A migration file + its entity changes = one commit
- Test files go with the production code they test
- Config changes that enable a feature go with that feature
- Unrelated formatting/lint fixes can be a separate commit

If everything belongs to one feature, make one commit. Don't split artificially.

### 4. Format Dart/Flutter code

If any Dart files (`*.dart`) are among the changes, run `dart format` **before** staging so the formatter's edits are included in the commit rather than left behind as a dirty working tree:

```bash
cd packages/app && dart format .
```

Why this comes before staging: if you stage first and then format, the formatter modifies the already-staged files on disk and you end up committing an unformatted version plus leaving formatted changes unstaged. Running format first avoids that split.

After formatting, re-check `git status` — the formatter may have touched files you weren't originally planning to commit. Only include files that are part of the intended commit groups.

### 5. Run quality checks

Before committing, run lint and typecheck using `nx affected` to keep it fast:

```bash
npx nx affected --target=lint --base=HEAD
npx nx affected --target=typecheck --base=HEAD
```

If either fails:
1. Attempt to fix the issues automatically (auto-fixable lint errors, type errors you can resolve)
2. Use a sub-agent for non-trivial fixes if needed
3. After fixing, re-run the checks to confirm they pass
4. Stage any files modified by the fix process

If checks pass, proceed to commit.

### 6. Commit each group

For each feature group:

1. Stage the relevant files by name (never `git add -A` or `git add .`)
2. Write a commit message following this format:

```
feat: <concise description of what changed>

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
```

**Message rules:**
- Format is `<type>: <description>` — no scope parentheses
- First line under 72 characters
- Use imperative mood ("add", "fix", "update", not "added", "fixes")
- Focus on **why**, not **what** — the diff shows the what
- If the change is a bug fix, use `fix:` instead of `feat:`
- If it's a refactor with no behavior change, use `refactor:`
- If it's purely chore/config, use `chore:`

3. Always pass the message via HEREDOC:
```bash
git commit -m "$(cat <<'EOF'
feat: description here

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

### 7. Handle hook failures

The pre-commit hook runs `format`, `test`, and `typecheck`. If a hook fails:

1. Read the error output carefully
2. Fix the root cause — do not use `--no-verify`. Use a sub-agent for non-trivial fixes if needed.
3. If the formatter modified files, stage those files with `git add <specific-files>`
4. Create a **new** commit (never `--amend`, as the failed commit didn't happen)
5. Re-run the commit

### 8. Repeat until clean

After all groups are committed, run `git status` to verify. If there are still unstaged changes that should be committed, repeat the process. Continue until the working tree is clean or only intentionally-untracked files remain.

## Things you must never do

- Use `--no-verify` to skip hooks
- Use `git add -A` or `git add .`
- Amend a previous commit (unless the user explicitly asks)
- Commit `.env` files or credentials
- Push to remote (unless the user explicitly asks)
- Use `-i` flag (interactive mode) with any git command
