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
request with the [community]($community).

## Language and Compiler
- **Structure and Styling**
  - Fix show rule recursion
  - Fix show-set order
  - Fix show-set where both show and set affect the same kind of element
    (to set properties on elements that match a selector)
  - Ancestry selectors (e.g., within)
  - Custom elements (that work with set and show rules)
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
  - Implement emoji export
  - HTML export
  - EPUB export
  - Tagged PDF for Accessibility
  - PDF/A and PDF/X support
- **Text and Fonts**
  - Font fallback warnings
  - Proper foundations for i18n
  - Bold, italic, and smallcaps synthesis
  - Variable fonts support
  - Ruby and Warichu
  - Kashida justification
- **Scripting**
  - Function hoisting if possible
  - Get values of set rules
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
  - Table stroke customization
- **Numbering**
  - Relative counters, e.g. for figure numbering per section
  - Improve equation numbering
  - Fix issues with numbering patterns
  - Enum continuation
- **Layout**
  - Row span and column span in table
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
  - Basic, built-in image editor (cropping, etc.)
  - Color Picker
  - Symbol picker
  - GUI inspector for editing function calls
  - Preview autocomplete entry
  - Cursor in preview
  - Inline documentation
  - More export options
  - Preview in a separate window
- **Writing**
  - Spell check
  - Word count
  - Structure view
  - Pomodoro
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
  - On-Premise deployment
  - Mobile improvements
