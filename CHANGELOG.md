# Changelog

## 1.0.12 — 2026-10-23

- A newer version now appears as a card in a news dialog instead of a banner strip: the card lists the release notes newer than the running version, links to the download page, and Don't show again hides it until a still newer version is out
- Occasional yaiol announcements appear in the same dialog; Settings > General has a new Show news from yaiol switch that turns them off completely (the new-version card is not governed by it)
- With Use AMLP_ID tag on, linking a FLAC that carries no id to a card that already has a Suno link now writes the id from that link (before, it only worked when the link was pasted after the file was linked, so the result depended on which came first)
- The Settings Display tab is renamed General
- Number fields show the corrected value at once: a value typed outside the allowed range snaps to the nearest allowed number immediately instead of keeping the typed text
- Date fields show the accent frame on keyboard focus, like every other field

## 1.0.11 — 2026-10-02

- New Media setting, **Stored paths**: Absolute (the default, as before) or Relative. Relative stores each linked file's path, and each project's media folder, from a **media root folder** you choose. Moving your whole library to another drive then means changing the root in one place, not relinking every card. Switching shows an audit first: how many paths will be rewritten, and which files are already missing. Nothing is written until you accept, and going back to Absolute converts everything back the same way. Relative needs every path to lie inside the root, so the switch is refused while any file is outside it, and the audit lists which ones
- Changing the root while in Relative mode rewrites nothing: the stored paths simply resolve under the new root, and the audit shows how many of your files are found there before you accept
- Editing a project's media folder now re-points only the cards whose file is missing, and tries the same sub-path under the new folder first. Renaming a folder on disk is therefore one edit on the project, and a card whose file still works is never moved by a folder edit. Files are still matched by name anywhere below the new folder when the sub-path doesn't exist
- JSON backups write media paths and project folders relative to the root while in Relative mode, so a backup moves to another machine without relocating anything

## 1.0.10 — 2026-09-30

- Reordering songs with **Rename media on sort** now carries the order into more than the file name. The song's `.lrc` lyrics file is renamed with it, so a lyrics file no longer becomes orphaned and the song no longer reads as having no lyrics file. A FLAC's embedded track number is also rewritten to its new position, keeping any zero-padding and any `/total` suffix. It is written even when the file name could not change because the target name was already taken, and a file whose tag already matches is not rewritten
- New Media setting, **Rename media on sort by default**, sets the starting value of the Sort view's Rename media on sort checkbox. That checkbox is now per session and starts from the setting each time, instead of remembering its last state
- Dragging a song in the Sort view now scrolls the list when the pointer nears its top or bottom edge, so a song can be carried to the first or last position in one drag. Before, anything off screen could only be reached in two steps
- Each song card in the Sort view has a play button, so you can listen while ordering. It stays usable on a finalized project, where the rest of the view is locked
- Actions now sit at the level of what they act on. Hovering a project row shows its own toolbar: edit, tag, translate, import folder, move to another environment and delete, without selecting the project first. While cards are selected, the strip above the list carries tag, translate, unlink from the project, move to another environment and delete for those cards. The left-hand bar keeps only the actions on the whole list: sort, new folder, sync music URLs, export to Excel and create a document
- Tag, Translate and Move each used to act on the selection or else on the whole open project, and Delete on the project when nothing was selected. Each now acts on exactly the target you started it from, and unlink no longer offers to remove every card of a project when nothing is selected
- The strip's **Select all** chip now reads **All**

## 1.0.9 — 2026-09-27

- Settings is now a full-window page with a side rail of tabs instead of a dialog. The header's settings button opens and closes it, and Esc leaves it. The app stays loaded underneath, so a playing track keeps playing
- Song, Music and Vocal cards gain two fields on their Style tab: **Reference artist** and **Reference complement**. They record what the style is aiming at: the act, plus an LP, song, era or genre that narrows it. They are kept apart from the style text because Suno removes contemporary artist names. Backups, environment copies and `.amlp` files (`REFERENCE ARTIST:` / `REFERENCE COMPLEMENT:`) all carry them
- `.amlp` files can give a track its own `LANG:` that overrides the file's language, and can include `TITLE (xx):` / `LYRICS (xx):` blocks. Import turns those blocks into the song's translations, and export writes them back out, along with a track's `LANG:` when it differs from the file's
- New Media setting, off by default: when a FLAC is linked, the Suno song id moves out of its COMMENT tag into a dedicated `AMLP_ID` tag. Pasting a Suno link onto a card writes the id into the linked FLAC when the file has none, which is the only way to identify a Suno Studio export, since Suno strips the id on export. Only that one metadata block is rewritten: audio and cover art are copied through unchanged, WAV files are never touched, and an id already in the file is never overwritten by a pasted link
- A card now holds at most one Suno link. Adding another replaces it, and linking a file whose Suno id differs corrects the card's link instead of adding a second one, which is what made a card export as the wrong song
- Bulk delete no longer leaves every card on screen until a reload when one of them fails. Cards that could not be deleted stay selected and the notice gives the reason, and a card that was already gone counts as deleted. Deleting a card or a project also removes it from the selection, so a later bulk action no longer asks for cards that are gone
- A failed `.amlp` import now says what went wrong: no tracks found, a file that could not be read, or the server's own one-line reason
- Notices raised while a dialog is open show above it instead of blurred behind it
- Copying a card into another environment keeps its description and note, which were dropped before
- Dropdown fields reopen on the next click after a pick, without clicking away first
- Input fields use a lighter background in the dark theme
- The header and About show the build number after the version, and the installer file name carries it too
- The custom icons (LRC Editor, and the copy title/style/lyrics glyphs) are built from editable SVG files via vite-plugin-svgr
- README rewritten, with a screenshot, install notes and an architecture overview
- New strings translated in all 52 languages

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
