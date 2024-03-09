---
description: What we have planned for Typst.
---

# Roadmap
This page lists planned features for the Typst language, compiler, library and
web app. Since priorities and development realities change, this roadmap is not
set in stone. Features that are listed here will not necessarily be implemented
and features that will be implemented might be missing here. As for bug fixes,
this roadmap will only list larger, more fundamental ones.

Are you missing something on the roadmap? Typst relies on your feedback as a
user to plan for and prioritize new features. Get started by filing a new issue
on [GitHub](https://github.com/typst/typst/issues) or discuss your feature
request with the [community].

## Language and Compiler
- **Structure and Styling**
  - Support for freezing content, so that e.g. numbers in it remain the same
    if it appears multiple times
  - Support for revoking style rules
  - Ancestry selectors (e.g., within)
  - Possibly a capability system, e.g. to make your own element referenceable
- **Layout**
  - Advanced floating layout
  - Rework layout engine to a more flexible model that has first-class support
    for both "normal" text layout and more canvas-like layout
  - Unified layout primitives across normal content and math
  - Named alignment to synchronize alignment across different layout hierarchies
  - Chained layout regions
  - Page adjustment from within flow
  - Advanced page break optimization
  - Grid-based typesetting
  - Layout with collision
- **Export**
  - Support for emojis in PDF
  - HTML export
  - EPUB export
  - Tagged PDF for Accessibility
  - PDF/A and PDF/X support
- **Text and Fonts**
  - Font fallback warnings
  - Bold, italic, and smallcaps synthesis
  - Variable fonts support
  - Ruby and Warichu
  - Kashida justification
- **Scripting**
  - Custom types (that work with set and show rules)
  - Function hoisting if possible
  - Doc comments
  - Type hints
- **Visualization**
  - Arrows
  - Better path drawing
  - Color management
- **Tooling**
  - Autoformatter
  - Linter
  - Documentation generator
- **Development**
  - Benchmarking
  - Better contributor documentation

## Library
- **Customization**
  - Richer built-in outline customization
- **Numbering**
  - Relative counters, e.g. for figure numbering per section
  - Improve equation numbering
  - Fix issues with numbering patterns
  - Enum continuation
- **Layout**
  - Balanced columns
  - Drop caps
  - End notes, maybe margin notes
- **Math**
  - Fix syntactic quirks
  - Fix font handling
  - Provide more primitives
  - Big fractions
- **Other**
  - Plotting

## Web App
- **Editing**
  - Smarter & more action buttons
  - Inline documentation
  - Preview autocomplete entry
  - Go-to-definition
  - Color Picker
  - Symbol picker
  - Basic, built-in image editor (cropping, etc.)
  - GUI inspector for editing function calls
  - Cursor in preview
- **Writing**
  - Spell check
  - Outline panel
  - Word count
  - Structure view
  - Text completion by LLM
- **Collaboration**
  - Chat-like comments
  - Change tracking
  - Version history
  - Git integration
- **Project management**
  - Drag-and-drop for projects
  - Thumbnails for projects
  - Template generation by LLM
- **Settings**
  - Keyboard shortcuts configuration
  - Better project settings
  - Avatar Cropping
- **Other**
  - Offline PWA
  - Single sign-on
  - Two-Factor Authentication
  - Advanced search in projects
  - Private packages in teams
  - Mobile improvements
