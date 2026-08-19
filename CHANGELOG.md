# Changelog

## 1.0.6 — 2026-08-19

- Import a whole LP prompt file: an `.amlp` becomes a project with one Song card per track, carrying each track's title, number, description, style, lyrics, language and date — from the Import button, by dropping the file on the window, or by double-clicking it on the desktop. It lands in the project you have open, or brings its own when you have none
- Export an LP file back out from the Export menu — the songs you have ticked, or everything the current view lists
- LP prompt files now carry the `.amlp` extension (they were `.suno`)
- New card from a Suno link: copy a song link and hit New (or Ctrl+N) — the card opens with its title, sort number, style, lyrics, date and URL already filled in
- Ctrl+N opens a new card, matching the header's + button
- The app starts faster and no longer flashes English before your own language: only the active language is loaded, and the window no longer paints black before the first render on a light theme
- The "open in LRC Editor" button now shows the LRC Editor icon instead of the same disc glyph the media controls use
- That button now greys out when LRC Editor is not installed — clicking it used to do nothing at all, with no explanation
- The Untagged row's count badge and the project drag ghost follow the theme instead of staying dark on a light one
- Replace the remaining hardcoded colours with theme tokens, adding `--tag` and `--env` for the tag and environment identity colours
- Add cold-start instrumentation behind `YAIOL_STARTUP_LOG`
- Adopt shared catalog v1 → v2 — on-icon count badges and popover form rows; neither is rendered here yet

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
