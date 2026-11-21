//! engine — everything that is not the window.
//!
//! No UI dependency: the app crate depends on this, never the reverse, so the toolkit stays
//! replaceable. Pure functions and data only.

pub mod dates;
pub mod excel;
pub mod ai;
pub mod aiid;
pub mod amlp;
pub mod cards;
pub mod docx;
pub mod drafts;
pub mod exports;
pub mod extras;
pub mod flac;
pub mod langs;
pub mod m4a;
pub mod media;
pub mod projects;
pub mod server;
pub mod songpage;
pub mod soundcloud;
pub mod store;
pub mod transfer;
pub mod translate;
pub mod wizard;

use std::cmp::Ordering;

/// Languages whose text runs right to left, by workspace language code.
pub fn is_rtl(lang: &str) -> bool {
    matches!(lang, "ar" | "he" | "fa" | "ur")
}

/// Compare two dotted versions ("1.2.3" vs "1.2.10") number by number. Missing segments are 0 and
/// a non-numeric segment reads as 0, so a malformed beacon can never panic - it just compares low.
/// The twin of `cmpVersion` in the shared update-check.js; the two must agree.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |v: &str| -> Vec<u64> {
        v.split('.').map(|p| p.trim().parse::<u64>().unwrap_or(0)).collect()
    };
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let o = pa.get(i).copied().unwrap_or(0).cmp(&pb.get(i).copied().unwrap_or(0));
        if o != Ordering::Equal {
            return o;
        }
    }
    Ordering::Equal
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_to_left_languages() {
        for code in ["ar", "he", "fa", "ur"] {
            assert!(is_rtl(code), "{code}");
        }
    }

    #[test]
    fn left_to_right_languages_and_variants() {
        for code in ["en", "fr", "zh_CN", "pt_BR"] {
            assert!(!is_rtl(code), "{code}");
        }
    }

    #[test]
    fn versions_compare_numerically_not_textually() {
        assert_eq!(compare_versions("1.2.10", "1.2.3"), Ordering::Greater);
        assert_eq!(compare_versions("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.9.9", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn malformed_versions_never_panic() {
        assert_eq!(compare_versions("", "0.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("abc", "1.0.0"), Ordering::Less);
    }


}
