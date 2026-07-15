// AppHeader - the ONE canonical header for every yaiol electron app. Reference: .template.
// Owns the invariant header skeleton: app name on the left, the version badge beside it, and
// the 2px gradient identity separator below (GUI Standard Rules 9 & 10). Each app passes ONLY
// its own header content as children (search box, view toggles, action-button groups). The
// name / badge / separator can never drift, because there is exactly ONE definition of them -
// here - rendered identically in every app. Its classes (.barh-header / .barh-app-name /
// .barh-app-version / .barh-gradient) come from the shared ui-app.css, so only color changes
// by theme, never shape.
//
// Distributed into each app's src/lib/ui-header.jsx by the workspace sync script - ⚠ SYNCED
// FILE, never edit the per-app copy; edit this canonical source and re-sync so every app's
// header stays byte-identical. Import: `import { AppHeader } from './lib/ui-header';`
//
// Header layout standard: buttons flow LEFT after the name/badge; only the standard
// source·help·settings group is pushed RIGHT (put a single `<span className="bar-spacer" />`
// immediately before that group). Then the 2px gradient separator; then an OPTIONAL
// sub-bar below it (a tag / filter strip), passed as `subBar` — so the order is always
// "header buttons → separator → optional sub-bar".
//
// Usage:
//   <AppHeader appName={APP_NAME} appVersion={APP_VERSION}
//              subBar={<>{tags}</>}>            {/* optional — omit if no sub-bar */}
//     <div className="barh-grp"> ...left buttons... </div>
//     <span className="bar-spacer" />
//     <div className="barh-grp"> ...github · help · settings... </div>
//   </AppHeader>
// Everything after the version badge is the app's own content, passed as children.
import React from 'react';

export function AppHeader({ appName, appVersion, children, subBar }) {
  return (
    <>
      <div className="barh-header">
        <span className="barh-app-name">{appName}</span>
        <span className="barh-app-version">v{appVersion}</span>
        {children}
      </div>
      {/* 2px gradient identity bar - GUI Standard Rule 10. */}
      <div className="barh-gradient" />
      {/* Optional secondary bar BELOW the separator — a tag / filter strip. Kept out of the
          header row so the separator sits between the buttons and the tags. */}
      {subBar != null && <div className="barh-subbar">{subBar}</div>}
    </>
  );
}
