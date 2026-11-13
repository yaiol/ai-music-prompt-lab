// UisAppHeader - the ONE canonical header for every yaiol electron app. Reference: .template.
// Owns the invariant header skeleton: app name on the left, the version badge beside it, and
// the 2px gradient identity separator below (GUI Standard Rules 9 & 10). Each app passes ONLY
// its own header content as children (search box, view toggles, action-button groups). The
// name / badge / separator can never drift, because there is exactly ONE definition of them -
// here - rendered identically in every app. Its classes (.uis-barh-header / .uis-barh-app-name /
// .uis-barh-app-version / .uis-barh-gradient) come from the shared uis-app.css, so only color changes
// by theme, never shape.
//
// Distributed into each app's src/lib/uis-header.jsx by the workspace sync script - ⚠ SYNCED
// FILE, never edit the per-app copy; edit this canonical source and re-sync so every app's
// header stays byte-identical. Import: `import { UisAppHeader } from './lib/uis-header';`
//
// Header layout standard: buttons flow LEFT after the name/badge; only the standard
// source·help·settings group is pushed RIGHT (put a single `<span className="uis-bar-spacer" />`
// immediately before that group). Then the 2px gradient separator; then an OPTIONAL
// sub-bar below it (a tag / filter strip), passed as `subBar` — so the order is always
// "header buttons → separator → optional sub-bar".
//
// Usage:
//   <UisAppHeader appName={APP_NAME} appVersion={APP_VERSION}
//              subBar={<>{tags}</>}>            {/* optional — omit if no sub-bar */}
//     <div className="uis-barh-grp"> ...left buttons... </div>
//     <span className="uis-bar-spacer" />
//     <div className="uis-barh-grp"> ...github · help · settings... </div>
//   </UisAppHeader>
// Everything after the version badge is the app's own content, passed as children.
//
// `bare`: an app that draws its own window bar (UisBarWindow) shows the name and the version THERE, so its
// header is rendered without them - the children start at the bar's left edge. Nothing else changes.
import React from 'react';

export function UisAppHeader({ appName, appVersion, children, subBar, bare = false }) {
  return (
    <>
      <div className="uis-barh-header">
        {!bare && <span className="uis-barh-app-name">{appName}</span>}
        {!bare && <span className="uis-barh-app-version">v{appVersion}</span>}
        {children}
      </div>
      {/* 2px gradient identity bar - GUI Standard Rule 10. */}
      <div className="uis-barh-gradient" />
      {/* Optional secondary bar BELOW the separator — a tag / filter strip. Kept out of the
          header row so the separator sits between the buttons and the tags. */}
      {subBar != null && <div className="uis-barh-subbar">{subBar}</div>}
    </>
  );
}
