# Changelog

## 1.0.5 — 2026-08-07
- Menu submenus now open reliably — they were clipped out of sight and unreachable whenever the menu was long enough to scroll
- Only one submenu stays on screen when sweeping down a menu, and it survives the pointer travelling from the row into it
- Submenus open beside their row and flip to the other side when the window edge is close
- Dropdown panels now use the height actually available on screen instead of a fixed 320px cap, so long menus no longer scroll for no reason
- Dropdowns only flip upwards when there is genuinely more room above

## 1.0.4 — 2026-08-02
- Help button now opens the app's own help page — it pointed at a stale address and led nowhere
- Derive the help URL from the app id in `package.json` instead of a typed literal, so it can no longer drift

## 1.0.3 — 2026-07-22
- New wand button in the media panel auto-links files to songs when both the track number and title match (flac preferred over wav)
- Linking a media file that carries Suno provenance automatically adds the suno.com song URL to the card
- Document-export dialog shows a live count of songs the export will include and disables Create when it is zero
- Export filters (lyrics / published) are now applied before the save dialog, with a clearer error when no songs match

## 1.0.2 — 2026-07-22
- Deleting a project now offers a checkbox to also delete all cards inside it
- `.suno` LP import reads each track's DESCRIPTION field into the card description
- Document export falls back to a `folder.jpg` beside the audio file when no cover art is embedded
- Restyle the document-export lyrics options as a standard button group

## 1.0.1 — 2026-07-22
- Add a "Copy title" button to every card, alongside copy style and copy lyrics
- Title copy uses the sort-number prefix (e.g. `01-My Song`) when the card is numbered
- Distinguish the three copy buttons with lettered T / S / L badges on the copy icon

## 1.0.0 — 2026-07-15

- Initial release
