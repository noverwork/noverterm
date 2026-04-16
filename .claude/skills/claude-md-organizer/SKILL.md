---
name: claude-md-organizer
description: "Organize and validate CLAUDE.md after any modification. MUST trigger whenever CLAUDE.md is edited, updated, or has new content added — including when adding new conventions, updating existing rules, or removing outdated sections. This skill ensures the document stays well-structured, consistent, and free of contradictions. Trigger on any mention of 'update CLAUDE.md', 'add to CLAUDE.md', 'write to CLAUDE.md', or after any Edit/Write operation on CLAUDE.md."
---

# CLAUDE.md Organizer

When CLAUDE.md is modified, the document can accumulate inconsistencies — duplicate rules, contradictory guidance, outdated references to files that no longer exist, or sections that grew out of order. This skill ensures every edit leaves the document in a clean, trustworthy state.

## When to run

After ANY modification to CLAUDE.md:
- Adding a new section or convention
- Updating an existing rule
- Removing content
- Reorganizing sections

## Workflow

### Step 1: Read the full CLAUDE.md

Read the entire file to understand the current state. Pay attention to:
- Section hierarchy (is the heading level correct?)
- Table of contents flow (do sections follow a logical order?)
- Cross-references (do they point to things that still exist?)

### Step 2: Check for issues

Scan for these specific problems:

**Structural issues:**
- Duplicate or near-duplicate content in different sections
- Heading levels that skip (e.g., `##` followed by `####` without `###`)
- Sections that grew too long and should be split
- Orphaned sub-sections that no longer belong under their parent

**Content consistency:**
- Contradictory rules (e.g., "use X" in one place, "don't use X" in another)
- Outdated file paths referencing files that were renamed, moved, or deleted
- Stale code examples that no longer match the actual implementation
- Token/color/typography references that don't match `app_colors.dart` or `app_typography.dart`

**Style consistency:**
- Mixed languages within the same section (Chinese and English should be intentional, not accidental)
- Inconsistent formatting (some rules use tables, similar rules use bullet lists)
- Missing examples where other similar sections have them

### Step 3: Fix issues found

For each issue:
1. Describe what's wrong (briefly)
2. Fix it in the file
3. Verify the fix doesn't break other references

### Step 4: Verify the result

After all fixes:
- Re-read the modified sections to confirm they're coherent
- Ensure no content was accidentally lost during reorganization
- Check that the document still flows logically from top to bottom

## Section order convention

CLAUDE.md follows this top-level structure:

```
1. Nx Configuration (auto-managed, don't touch)
2. Product Overview (Truley Sentinel description)
3. Project Structure
4. Tech Stack
5. Package Manager & Commands
6. TypeScript Configuration
7. Module Boundaries
8. Rust Conventions
9. Backend Conventions
10. Flutter App Conventions (largest section)
    - 發佈平台
    - 導航 (Mobile + Desktop)
    - 架構與跨平台
    - 產品特性
    - 資料與狀態
    - 程式碼品質 (naming, i18n, theme, color, typography, hover, testing)
11. Shared Package Conventions
12. Dev Infrastructure
13. Environment Variables
```

New Flutter conventions should go under section 10, in the appropriate sub-section. Don't create new top-level sections unless the content truly doesn't fit anywhere.

## What NOT to change

- The `<!-- nx configuration start-->` ... `<!-- nx configuration end-->` block — this is auto-managed
- Content that was just deliberately added by the user in the current conversation — don't "organize" away fresh changes
- Wording that reflects deliberate product decisions (e.g., specific color values, spacing scales)
