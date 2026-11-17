//! Map solver `ChartOutOfRange` labels to input fields for highlight copy.

use da20_core::chart_bounds::ChartOutOfRange;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ChartInputField {
    SharedPressureAlt,
    SharedOat,
    TakeoffWeight,
    TakeoffSignedWind,
    TakeoffObstacle,
    Wind,
    ClimbWeight,
    ClimbTargetAlt,
    CruisePa,
    CruiseOat,
    CruiseGroundElev,
    CruiseGroundOat,
    CruiseRpm,
    CruisePctBhp,
    LandingPa,
}

pub fn chart_ooc_matches_field(ooc: &ChartOutOfRange, field: ChartInputField) -> bool {
    let label = ooc.label;
    match field {
        ChartInputField::SharedPressureAlt | ChartInputField::LandingPa => {
            label.contains("PA") || label.contains("pressure altitude") || label == "Table 4 PA"
        }
        ChartInputField::SharedOat
        | ChartInputField::CruiseOat
        | ChartInputField::CruiseGroundOat => {
            label.contains("OAT") || label.contains("ΔISA") || label == "Table 3 ΔISA"
        }
        ChartInputField::TakeoffWeight | ChartInputField::ClimbWeight => label.contains("weight"),
        ChartInputField::TakeoffSignedWind | ChartInputField::Wind => {
            label.contains("tailwind") || label.contains("headwind")
        }
        ChartInputField::TakeoffObstacle => label.contains("obstacle"),
        ChartInputField::ClimbTargetAlt | ChartInputField::CruisePa => {
            label.contains("PA") || label.contains("pressure altitude")
        }
        ChartInputField::CruiseGroundElev => label.contains("PA") && label.contains("Fig 5.2"),
        ChartInputField::CruiseRpm => label.contains("RPM"),
        ChartInputField::CruisePctBhp => label.contains("BHP") || label.contains("%BHP"),
    }
}

pub fn takeoff_wind_axis_limits(locale: crate::i18n::Locale) -> String {
    let c = crate::i18n::copy(locale);
    c.takeoff_wind_chart_limits.to_string()
}

pub fn takeoff_obstacle_axis_limits(locale: crate::i18n::Locale) -> String {
    let c = crate::i18n::copy(locale);
    c.takeoff_obstacle_chart_limits.to_string()
}

pub fn field_ooc_active(ooc: Option<&ChartOutOfRange>, field: ChartInputField) -> bool {
    ooc.is_some_and(|e| chart_ooc_matches_field(e, field))
}
