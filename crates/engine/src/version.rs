//! Compile-checked CMS-HCC model versioning.
//!
//! The model version is a *type-level* dimension: risk tables and the engine are
//! generic over a [`ModelVersion`] marker, so coefficient/mapping tables loaded for
//! one version cannot be combined with an engine of another version without a
//! compile error. v1 pins **V28** only (per project scope); adding V24 later means
//! adding a marker type, not refactoring call sites.

use std::fmt::Debug;

/// Marker trait for a CMS-HCC model version.
pub trait ModelVersion: Debug + Copy + Default + 'static {
    /// Short model identifier, e.g. `"V28"`.
    const NAME: &'static str;
    /// Payment year the model is operative for (drives the age cutoff date).
    const PAYMENT_YEAR: i32;
    /// Age is computed as of February 1 of the payment year (CMS convention).
    fn cutoff_date() -> super::demographics::Date {
        super::demographics::Date { year: Self::PAYMENT_YEAR, month: 2, day: 1 }
    }
}

/// CMS-HCC **V28**, operative for payment year 2026. The only model implemented in v1.
#[derive(Debug, Clone, Copy, Default)]
pub struct V28;

impl ModelVersion for V28 {
    const NAME: &'static str = "V28";
    const PAYMENT_YEAR: i32 = 2026;
}
