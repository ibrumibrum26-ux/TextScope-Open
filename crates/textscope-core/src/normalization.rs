use unicode_normalization::{is_nfc, is_nfd, is_nfkc, is_nfkd};

use crate::NormalizationInspection;

pub(crate) fn inspect_normalization(text: &str) -> NormalizationInspection {
    NormalizationInspection {
        nfc_would_change: !is_nfc(text),
        nfd_would_change: !is_nfd(text),
        nfkc_would_change: !is_nfkc(text),
        nfkd_would_change: !is_nfkd(text),
    }
}
