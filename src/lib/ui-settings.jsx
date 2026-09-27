// SettingsView - the ONE settings page for every yaiol electron app. Reference: .template.
// Settings is a full-window VIEW, not a dialog: it takes everything below the header - it covers
// the app's content region (.app-main) and, while open, the catalog footer bars are hidden (the
// stg-open effect below). The app underneath stays MOUNTED, only hidden, so a playing track (whose
// <audio> may live inside the footer), an editor's undo history or a scroll position survive a
// visit to settings. A left rail (the catalog's .tabs.vertical) lists the tabs; the
// active tab fills the rest of the window.
//
// What this component OWNS, identically in every app:
//   - the frame: rail + page, the title, Esc to leave
//   - the top of Display: Language + Theme
//   - the top of About: logo, name, version, the app's one-line description
// What each app PASSES:
//   - displayExtra : fields rendered BELOW Language + Theme
//   - tabs         : the app's own tabs, [{ key, label, icon, content }], placed between
//                    Display and About
//   - aboutExtra   : content rendered BELOW the About block
// ⚠ CLAUDE: an app ADDS after the common part, it never replaces or reorders it. An app that
// needs Theme gone or moved is a catalog decision, not a local override.
//
// Changes apply live (every setter persists immediately), so leaving is plain navigation:
// the back arrow beside the title, the header's settings button (a toggle) and Esc all leave it.
// While open, the rest of the header is inert (ui-app.css, under .stg-open) - so the app's settings
// button MUST carry the class `stg-toggle`, the one header control exempted.
//
// Mount it as the FIRST child of the app's .app-main (the one position:relative content region)
// while settings is open; .stg-view covers that region absolutely.
//
// Distributed into each app's src/lib/ui-settings.jsx by the workspace sync script - ⚠ SYNCED
// FILE, never edit the per-app copy; edit this canonical source and re-sync.
// Import: `import { SettingsView } from './lib/ui-settings';`
import React, { useEffect, useRef, useState } from 'react';
import { ArrowLeft, Sun, Moon, ScrollText } from 'lucide-react';
import yaiolLogo from '../assets/yaiol-logo.svg';

export function SettingsView({
  t, appName, appVersion, languages,
  lang, setLang, theme, setTheme,
  displayExtra, tabs = [], aboutExtra,
  onClose,
}) {
  const [activeTab, setActiveTab] = useState('display');
  // Tabs visited so far. A tab mounts the first time it is shown and then stays mounted:
  // a heavy tab (icons-cockpit's Libraries fetches every icon set) costs nothing until it is
  // opened, and a tab's local state survives switching away and back.
  const [visited, setVisited] = useState(() => new Set(['display']));
  const showTab = (key) => { setActiveTab(key); setVisited(v => v.has(key) ? v : new Set(v).add(key)); };

  // While the page is open, <html> carries .stg-open and ui-app.css hides the footer bars
  // (.barh-footer, .bar-status). On <html>, not on .app-root, because a footer can be PORTALED
  // outside the app root (ampl's player). Hidden with CSS, never unmounted: a footer may own the
  // <audio> element, and unmounting it would stop the track.
  useEffect(() => {
    document.documentElement.classList.add('stg-open');
    return () => document.documentElement.classList.remove('stg-open');
  }, []);

  // The KEYBOARD is the page's too, as the mouse is. Every key event is stopped on <body>: that is
  // AFTER React has delivered it to the focused control (React listens on the root container, inside
  // <body>), so typing in a settings field works, and BEFORE the app's own shortcut listeners, which
  // all sit on document / window - so no app shortcut (markzen's Ctrl+S, lrce's playback keys) can
  // act on the view the page is covering. Focus moves into the page and Tab is kept inside it, so
  // nothing behind it can be reached from the keyboard either.
  // Escape leaves the page. While a POPUP opened from a settings tab is up (a combobox or menu
  // dropdown, a date picker - they close on their own document listener), Escape closes that popup
  // instead: it is let through to document, then stopped there before window. Every other key stays
  // blocked - a popup handles arrows / Enter on its own element, before <body>.
  // ⚠ CLAUDE: this is the ONE mechanism. No app adds a `settingsOpen` check to its own shortcut
  // handler - while the page is open that handler is never reached, popup or not. A settings tab
  // never opens a full dialog (none does); if one ever must, extend THIS function first.
  const viewRef = useRef(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;
  useEffect(() => {
    const view = viewRef.current;
    view.querySelector('.tab.active')?.focus();
    const POPUP = '.pop-surface, .dp-pop';
    const focusables = () => [...view.querySelectorAll('button, input, select, textarea, a[href], [tabindex]:not([tabindex="-1"])')]
      .filter(el => !el.disabled && el.offsetParent !== null);
    const onKey = (e) => {
      const popup = document.querySelector(POPUP);
      if (popup && e.key === 'Escape') return;   // on to document, where the popup closes itself
      e.stopPropagation();
      if (e.type !== 'keydown') return;
      if (e.key === 'Escape') { onCloseRef.current(); return; }
      if (e.key !== 'Tab') return;
      const els = focusables();
      if (!els.length) return;
      const first = els[0], last = els[els.length - 1], at = document.activeElement;
      if (!view.contains(at)) { e.preventDefault(); first.focus(); }
      else if (e.shiftKey && at === first) { e.preventDefault(); last.focus(); }
      else if (!e.shiftKey && at === last) { e.preventDefault(); first.focus(); }
    };
    // The Escape a popup was given stops at document: the popup's own document listener still runs
    // (stopPropagation spares listeners on the same node), the app's window listeners do not.
    const onDocKey = (e) => { if (e.key === 'Escape' && document.querySelector(POPUP)) e.stopPropagation(); };
    const TYPES = ['keydown', 'keyup', 'keypress'];
    TYPES.forEach(t => { document.body.addEventListener(t, onKey); document.addEventListener(t, onDocKey); });
    return () => TYPES.forEach(t => { document.body.removeEventListener(t, onKey); document.removeEventListener(t, onDocKey); });
  }, []);

  const display = (
    <>
      <div className="dlg-field">
        <label className="dlg-field-label">{t('lblStgDisplayLang')}</label>
        <select className="select" value={lang} onChange={e => setLang(e.target.value)}>
          {languages.map(l => <option key={l.key} value={l.key}>{l.label}</option>)}
        </select>
      </div>
      <div className="dlg-field divider">
        <label className="dlg-field-label">{t('lblStgDisplayTheme')}</label>
        <div className="opt-btns">
          {[
            { key: 'dark',  Icon: Moon, label: t('btnStgDisplayThemeDark') },
            { key: 'light', Icon: Sun,  label: t('btnStgDisplayThemeLight') },
          ].map(({ key, Icon, label }) => (
            <button key={key} className={`opt-btn ${theme === key ? 'active' : ''}`} onClick={() => setTheme(key)}>
              <Icon />
              <span>{label}</span>
            </button>
          ))}
        </div>
      </div>
      {displayExtra}
    </>
  );

  const about = (
    <>
      <div className="dlg-about">
        <img className="dlg-about-logo" src={yaiolLogo} alt="yaiol" />
        <div className="dlg-about-text">
          <div className="dlg-about-id">{appName} <b>v{appVersion}</b> by yaiol</div>
          <div className="dlg-about-desc">{t('msgStgAboutDesc')}</div>
        </div>
      </div>
      {aboutExtra}
    </>
  );

  const allTabs = [
    { key: 'display', label: t('tabStgDisplay'), icon: Sun, content: display },
    ...tabs,
    { key: 'about', label: t('tabStgAbout'), icon: ScrollText, content: about },
  ];

  return (
    <div className="stg-view" ref={viewRef}>
      <div className="stg-rail pnl">
        <div className="stg-head">
          {/* The back arrow is the page's visible way out (the header's settings toggle and Esc are the others). */}
          <button className="btn icon subtle" onClick={onClose} title={t('tipStgBack')} aria-label={t('tipStgBack')}><ArrowLeft /></button>
          <span className="dlg-title">{t('ttlStg')}</span>
        </div>
        <div className="tabs vertical">
          {allTabs.map(({ key, label, icon: TabIcon }) => (
            <button key={key} className={`tab ${activeTab === key ? 'active' : ''}`} onClick={() => showTab(key)}>
              <TabIcon />{label}
            </button>
          ))}
        </div>
      </div>
      <div className="stg-body dlg-form">
        {allTabs.filter(({ key }) => visited.has(key)).map(({ key, content }) => (
          <div key={key} className="stg-page" hidden={activeTab !== key}>{content}</div>
        ))}
      </div>
    </div>
  );
}
