use std::time::Duration;

/// Parses durations such as `250ms`, `30s` or `5m`.
pub fn parse_duration(raw: &str) -> Result<Duration, String> {
    let raw = raw.trim();
    let split = raw.find(|c: char| !c.is_ascii_digit()).unwrap_or(raw.len());
    let (digits, unit) = raw.split_at(split);
    let value: u64 = digits.parse().map_err(|_| format!("invalid duration: {raw:?}"))?;
    match unit {
        "ms" => Ok(Duration::from_millis(value)),
        "s" => Ok(Duration::from_secs(value)),
        "m" => Ok(Duration::from_secs(value * 60)),
        _ => Err(format!("unknown unit in {raw:?}, expected ms, s or m")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_units() {
        assert_eq!(parse_duration("250ms"), Ok(Duration::from_millis(250)));
        assert_eq!(parse_duration("30s"), Ok(Duration::from_secs(30)));
        assert_eq!(parse_duration("5m"), Ok(Duration::from_secs(300)));
        assert!(parse_duration("5d").is_err());
        assert!(parse_duration("ms").is_err());
    }
}
