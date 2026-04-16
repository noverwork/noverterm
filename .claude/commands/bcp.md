---
description: Create branch, commit changes, push and create PR
---

Execute the following workflow to create a branch, commit all changes, push to remote, and create a Pull Request:

1. **Analyze changes**: Run `git status` and `git diff` to understand what has been modified
2. **Create branch**: Create a new branch with an appropriate name based on the changes (use conventional naming like `feat/`, `fix/`, `refactor/`, `chore/`)
3. **Stage changes**: Stage all relevant changes with `git add`
4. **Run lint and autofix**: Run `npx eslint <changed-files> --fix` to automatically fix lint issues (especially import sort problems)
5. **Commit**: Create a commit with a descriptive message following conventional commit format
6. **Push**: Push the branch to origin with upstream tracking
7. **Create PR**: Use `gh pr create` to create a Pull Request with:
   - A clear title summarizing the changes
   - A body with a Summary section (bullet points) and Test plan section

Important:

- Infer the branch name and commit message from the actual changes - do not use placeholders
- If there are no changes to commit, inform the user and stop
- The PR should target the `main` branch
- Include the robot emoji footer in commit messages as per repository conventions
