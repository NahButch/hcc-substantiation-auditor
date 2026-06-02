//! Age calculation and age/sex variable assignment (CMS V28 conventions).

use crate::types::Sex;

/// A calendar date. Kept dependency-free (no `chrono`) so the engine has no
/// external crates — determinism and auditability are the priority here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Date { year, month, day }
    }
}

/// Age in completed years as of `cutoff`, mirroring the reference
/// `calculate_age` (subtract a year if the birthday has not occurred by cutoff).
pub fn calculate_age(dob: Date, cutoff: Date) -> i32 {
    let mut years = cutoff.year - dob.year;
    if cutoff.month < dob.month || (cutoff.month == dob.month && cutoff.day < dob.day) {
        years -= 1;
    }
    years
}

/// CMS V28 continuing-enrollee age bands (inclusive). `None` upper bound = open (95+).
pub const AGE_BANDS: &[(i32, Option<i32>)] = &[
    (0, Some(34)),
    (35, Some(44)),
    (45, Some(54)),
    (55, Some(59)),
    (60, Some(64)),
    (65, Some(69)),
    (70, Some(74)),
    (75, Some(79)),
    (80, Some(84)),
    (85, Some(89)),
    (90, Some(94)),
    (95, None),
];

/// The age/sex coefficient variable for a beneficiary, e.g. `F70_74`, `M95_GT`.
/// Returns the first band the age falls into (bands are non-overlapping).
pub fn age_sex_variable(age: i32, sex: Sex) -> String {
    let s = match sex {
        Sex::Male => 'M',
        Sex::Female => 'F',
    };
    for &(start, stop) in AGE_BANDS {
        let in_band = age >= start && stop.map_or(true, |hi| age <= hi);
        if in_band {
            return match stop {
                Some(hi) => format!("{s}{start}_{hi}"),
                None => format!("{s}{start}_GT"),
            };
        }
    }
    // Ages below 0 are invalid input; clamp to the youngest band defensively.
    format!("{s}0_34")
}

/// `DISABL`: under 65 and originally entitled by disability (OREC 1, 2, or 3).
pub fn is_disabled(age: i32, orec: u8) -> bool {
    age < 65 && matches!(orec, 1 | 2 | 3)
}

/// `ORIGDIS`: originally disabled but now aged (OREC == 1 and not currently disabled).
pub fn is_originally_disabled(age: i32, orec: u8) -> bool {
    orec == 1 && !is_disabled(age, orec)
}
