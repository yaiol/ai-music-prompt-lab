//! The yaiol UI catalog, Dioxus implementation.
//!
//! One catalog, two implementations: the CSS in `assets/` is the shared contract (copied from the
//! shared catalog, never edited here), and each control below emits the SAME class names and
//! element structure as its JSX twin in the shared catalog. Only the controls this app uses are
//! ported; the rest are `pending` — see the project's CLAUDE.md for the status table.

mod bar_window;
mod header;
mod icons;
pub mod news;
mod dlg_news;
mod news_loader;
mod markdown_renderer;
mod stg_view;

pub use bar_window::UisBarWindow;
pub use header::UisAppHeader;
pub use icons::UisIcoGithub;
pub use news_loader::{UisBtnNews, UisBtnUpdate, UisNewsLoader};
pub use stg_view::UisStgView;
mod btn_copy;
mod fld_value;
mod menu;
mod num;
mod pkr_color;
mod pkr_date;
mod cbo_standard;
mod pnl_splitter;
mod pnl_toggle;
mod popover;
pub mod toast;

pub use btn_copy::UisBtnCopy;
pub use fld_value::UisFldValue;
pub use menu::{UisMenu, UisMenuItem};
pub use num::UisNum;
pub use pkr_color::UisPkrColor;
pub use pkr_date::UisPkrDate;
pub use cbo_standard::{UisCboStandard, UisComboItem};
pub use pnl_splitter::UisPnlSplitter;
pub use pnl_toggle::UisPnlToggle;
pub use popover::UisPopover;
pub use stg_view::UisAppTab;
