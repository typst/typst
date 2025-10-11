# Compile Queue Design (Step 2 for Issue #86)

## Goal
Handle incoming `source_patch` messages and compile only the affected pages.

## Features
- Prioritize visible pages from `viewport` messages.
- Cancel old compilation tasks when new edits arrive.
- Compile other pages in background to keep interface smooth.

## API Plan
- `enqueue(patch_id, page, priority)`
- `cancel(patch_id)`
- `process_next()` executes highest-priority pending task.

## Next Steps
- Connect this queue to the websocket handler.
- Add logging and simple tests.
