//! Canonical wind display: direction then speed, zero-padded (e.g. `200 / 08 kt`).

/// Parse `200 / 08 kt` or `200/8` style wind strings.
pub fn parse_wind(s: &str) -> Option<(f64, f64)> {
    let t = s.trim().replace("kt", "").replace("KT", "");
    let parts: Vec<&str> = t.split('/').map(str::trim).collect();
    if parts.len() != 2 {
        return None;
    }
    let dir = parts[0].parse().ok()?;
    let spd = parts[1].parse().ok()?;
    Some((dir, spd))
}

pub fn format_wind(dir_deg: f64, speed_kt: f64) -> String {
    let mut dir = dir_deg.round().rem_euclid(360.0) as u32;
    if dir == 0 {
        dir = 360;
    }
    let spd = speed_kt.round().max(0.0) as u32;
    format!("{dir:03} / {spd:02} kt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn north_wind_shows_360() {
        assert_eq!(format_wind(0.0, 5.0), "360 / 05 kt");
        assert_eq!(format_wind(360.0, 5.0), "360 / 05 kt");
    }

    #[test]
    fn pads_direction_and_speed() {
        assert_eq!(format_wind(200.0, 8.0), "200 / 08 kt");
        assert_eq!(format_wind(9.0, 11.0), "009 / 11 kt");
    }
}
