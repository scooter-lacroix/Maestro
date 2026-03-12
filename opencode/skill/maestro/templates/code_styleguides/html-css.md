# HTML and CSS Guide

When I write HTML and CSS, I start with semantics, accessibility, and layout clarity before I worry about visual flourishes.

These rules are mandatory defaults for new code. I only break them when a project constraint is real, documented, and local.

## What I optimize for

- Accessible interfaces that work well with keyboard, screen reader, and touch input.
- Structure that stays understandable without the styles turned on.
- Styling systems that scale without selector wars.
- Responsive layouts that adapt cleanly instead of fighting the viewport.

## Required defaults

- Use semantic HTML first: buttons for actions, links for navigation, lists for collections, headings in a real hierarchy.
- Keep forms explicit with labels, validation messaging, and focus states that are easy to see.
- Use CSS custom properties for tokens such as spacing, color, radius, and typography.
- Prefer layout systems with intent: flex for one-dimensional flow, grid for two-dimensional structure.
- Start mobile-first and let complexity grow only where larger screens actually benefit.

## Architecture

- Keep class naming consistent and meaningful to the component or pattern, not to one temporary visual quirk.
- Limit selector depth so styles stay local and easy to override intentionally.
- Treat animation as part of UX: clear, restrained, and respectful of reduced-motion preferences.
- Keep visual tokens centralized so a theme change does not require a hunt across dozens of files.

## Verification

- Check empty, loading, error, and dense-content states, not just the perfect screenshot path.
- Test across viewport sizes and with long or translated content.
- Audit contrast, focus order, and semantic landmarks as part of normal review.
- Optimize expensive paints and layout thrash only when measurement shows a real problem.

## Explicitly prohibited

The following practices are prohibited in new code unless the guide names a narrow, explicit exception.

- Div soup when a semantic element already exists.
- Absolute positioning as a primary layout tool.
- Selectors that depend on brittle DOM structure or nth-child magic.
- Color-only communication, missing focus styles, and hover-only interactions.
