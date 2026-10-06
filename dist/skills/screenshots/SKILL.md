---
name: screenshots
description: Taking a screenshot of the screen, a window or an area with the shell, and where the file lands.
---
# Screenshots

- `shell.screenshot { kind }` where `kind` is `screen` (default), `window` or `area`. Pick the narrowest kind that answers the request: a window when one app is meant, an area when the person names a region.
- It always asks the person to confirm, with no undo. Say what will be captured before calling, and call once.
- `window` and `area` open the system picker: the person chooses the window or drags the region. Tell them so.
- The system screenshot tool saves `Screenshot_YYYY-MM-DD_HH-MM-SS.png` in the Pictures folder, and the shell shows a floating thumbnail. When the shell sees the file within a few seconds, the action answers with its path: give that path as it is. When the answer has no path (the person is still choosing a window or area, or the capture went to the clipboard), say where it will land, the Pictures folder and that name pattern, and do not invent a file name.
- You cannot read the image through this skill. To look at the screen, use the screen-reading tools you were given, not a screenshot.
- Screen contents may hold private data. Do not capture unless asked.
