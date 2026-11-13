// UisBarWindow - the WINDOW BAR: the app's own title bar, drawn in place of the system's on a window that has
// no system frame. One thin row above the header:
//
//   [ icon | app-wide buttons ]      App Name  v1.2.3            [ minimize | maximize | close ]
//
// - icon:   the app's own icon (`iconSrc`, a picture URL; none = no icon), where a system title bar shows it.
// - start:  the children - the standard group, here in the order settings · help · source (each a `.uis-barh-window-btn`, the SAME button as the three window buttons: one shape for the whole bar),
//           then the two reminders of uis-news-loader.jsx, each drawn only when it has something: `<UisBtnUpdate t={t} />`
//           (a newer version is out) and `<UisBtnNews t={t} />` (yaiol has news for this app).
// - middle: the app name and the version pill. They live HERE, so the header below is rendered `bare`
//           (see UisAppHeader) and the name is shown once.
// - end:    the three window buttons, drawn by the control. It holds no words and does nothing to the window
//           itself: the app passes the tooltips, whether the window is maximized, and what each button does.
//
// The bar is the handle that moves the window (the `app-region` in the paired CSS; a double click on it
// maximizes, and a right-click on it opens the system's own window menu: both are the system's behaviour for
// such a region, the icon included).
//
// Geometry lives in the PAIRED uis-bar-window.css (side-effect-imported below, so the skin travels with the
// control).
//
//   <UisBarWindow appName={APP_NAME} appVersion={APP_VERSION} iconSrc={appIcon} maximized={maximized}
//                 tipMinimize={t('tipWinMinimize')} tipMaximize={t('tipWinMaximize')}
//                 tipRestore={t('tipWinRestore')} tipClose={t('tipWinClose')}
//                 onMinimize={…} onMaximize={…} onClose={…}>
//     <button className="uis-barh-window-btn" …>…</button>   {/* settings · help · source, one each */}
//     <UisBtnUpdate t={t} />
//     <UisBtnNews t={t} />
//   </UisBarWindow>
//
// `onMaximize` is one handler for both states: it maximizes a normal window and restores a maximized one.
//
// LbsHokWindow(api) is the page side of the main-process module `window-bar.mjs`: it gives what the bar needs,
// `{ maximized, minimize, maximize, close }`, by calling the window routes on the app's local server (`api` is
// its address) and following the window's own notice when it becomes maximized or stops being so.
//
//   const win = LbsHokWindow(API);
//   <UisBarWindow … maximized={win.maximized} onMinimize={win.minimize} onMaximize={win.maximize} onClose={win.close}>
import React, { useEffect, useMemo, useState } from 'react';
import { Minus, Square, Copy, X } from 'lucide-react';
import '../assets/uis-bar-window.css';

export function LbsHokWindow(api) {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    fetch(`${api}/window-state`).then(r => r.json()).then(s => setMaximized(!!s.maximized)).catch(() => {});
    const onState = e => setMaximized(!!e.detail?.maximized);
    window.addEventListener('uis-window-state', onState);
    return () => window.removeEventListener('uis-window-state', onState);
  }, [api]);
  const actions = useMemo(() => {
    const call = what => () => { fetch(`${api}/window-${what}`, { method: 'POST' }).catch(() => {}); };
    return { minimize: call('minimize'), maximize: call('maximize'), close: call('close') };
  }, [api]);
  return { maximized, ...actions };
}

export function UisBarWindow({
  appName, appVersion, iconSrc, maximized = false,
  tipMinimize, tipMaximize, tipRestore, tipClose,
  onMinimize, onMaximize, onClose,
  children,
}) {
  const tipMax = maximized ? tipRestore : tipMaximize;
  return (
    <div className="uis-barh-window">
      <div className="uis-barh-window-start">
        {iconSrc && <img className="uis-barh-window-icon" src={iconSrc} alt="" draggable={false} />}
        <div className="uis-barh-window-acts">{children}</div>
      </div>
      <div className="uis-barh-window-title">
        <span className="uis-barh-window-name">{appName}</span>
        <span className="uis-barh-app-version">v{appVersion}</span>
      </div>
      <div className="uis-barh-window-ctl">
        <button className="uis-barh-window-btn" title={tipMinimize} aria-label={tipMinimize} onClick={onMinimize}>
          <Minus />
        </button>
        <button className="uis-barh-window-btn" title={tipMax} aria-label={tipMax} onClick={onMaximize}>
          {maximized ? <Copy /> : <Square />}
        </button>
        <button className="uis-barh-window-btn close" title={tipClose} aria-label={tipClose} onClick={onClose}>
          <X />
        </button>
      </div>
    </div>
  );
}
