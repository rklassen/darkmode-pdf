# Markdown Feature Specification (MDSpec)

## Target Flavor

This project targets:

1. **CommonMark** as the core syntax baseline.
2. **GitHub Flavored Markdown (GFM)** extensions.
3. **LaTeX math extension** for formulas.

## CommonMark Core Features

1. Paragraphs
2. ATX headings (`#`...`######`)
3. Setext headings (`===` / `---`)
4. Thematic breaks
5. Block quotes
6. Ordered and unordered lists
7. Fenced code blocks
8. Indented code blocks
9. Inline code spans
10. Emphasis and strong emphasis
11. Links (inline + reference)
12. Images (inline + reference)
13. Autolinks (`<https://...>`)
14. Raw HTML passthrough (subject to renderer policy)
15. Escapes/entities
16. Soft and hard line breaks
17. Link reference definitions

## GFM Extensions

1. Tables
2. Strikethrough (`~~text~~`)
3. Task list items (`- [ ]`, `- [x]`)
4. Autolink literals

## Math Extension (LaTeX)

Math formulae are authored in **LaTeX syntax**:

1. Inline math: `$...$`
2. Display math: `$$...$$`
3. Optional compatibility forms: `\(...\)` and `\[...\]`

## Notes

1. Markdown is not a single universal spec; this file defines the project contract.
2. Any future non-GFM extensions (footnotes, definition lists, MDX, etc.) must be explicitly added here.
