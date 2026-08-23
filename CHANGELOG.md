# Changelog

## 1.0.8 — 2026-08-23

- The folder picker reopens in the folder it last used instead of starting in Downloads, and remembers it across restarts — so pointing a project at its media folder, or importing a folder of tracks, lands where you were working
- The header help button and the update banner's What's new / Download links open the page in the app's own language. They had been addressed through a four-language allowlist that silently sent every other UI language to English; they are now directory URLs in the app's language, and the site's own fallback decides what to serve
- Update the dependencies — better-sqlite3 12 → 13, Electron 42 → 43, Vite 8.1 → 8.2, lucide-react 1.21 → 1.33, react-colorful 5.7 → 5.8, @electron/rebuild 4.0 → 4.2, music-metadata 11.13 → 11.15, plus patch bumps to React, @vitejs/plugin-react, concurrently, wait-on, cors, brace-expansion and picomatch
- Rename `vite.config.js` to `vite.config.mjs` — the package declares no `type: module`, so the ESM config has to announce itself by extension

## 1.0.7 — 2026-08-22

- Drop tracks on the window to create Song cards. One audio file opens the card editor filled in; several files — or a folder, at any depth — become cards straight away, in the order dropped, in the project you have open
- A dropped track that Suno rendered arrives complete: title, sort number, style, lyrics, date and the song's link. One it didn't still becomes a card linked to the file, its name split on the `NN-Title` pattern
- A dropped file naming a Suno song that has since been deleted or made private now creates nothing and reports how many were left out, rather than a card for a track that is no longer there
- Linking a Suno-rendered **mp3** now adds the song's suno.com link to the card. Suno stores that provenance in a user-defined comment frame, which the app never read — so the link only ever appeared for flac and wav
- A Suno page that answers 404 is treated as gone instead of scraped anyway. Suno serves a normal-looking generic page on a dead song, so the fetch used to succeed and produce a blank card
- The preload script is now actually attached to the window — it was written but never loaded, so its bridge did nothing — and exposes `webUtils.getPathForFile`, which is how a dropped file's location is known
- New toast string for skipped songs, translated in all 52 languages; the "cards created" string dropped `JSON` from its key now that it is not JSON-specific

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
