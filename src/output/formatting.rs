pub fn grouped(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    if value < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub fn percent(value: f64) -> String {
    format!("{value:.1}%")
}

pub fn bar(fraction: f64, width: usize, ascii: bool) -> String {
    let filled = ((fraction.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    let (fill, empty) = if ascii { ('#', '.') } else { ('█', '░') };
    let mut bar = String::with_capacity(width * 3);
    bar.extend(std::iter::repeat_n(fill, filled));
    bar.extend(std::iter::repeat_n(empty, width - filled));
    bar
}

pub fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let kept: String = text.chars().take(limit.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(1_234_567), "1,234,567");
        assert_eq!(grouped(-4_200), "-4,200");
    }

    #[test]
    fn truncates_on_character_boundaries() {
        assert_eq!(truncate("héllo wörld", 6), "héllo…");
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn bar_has_fixed_width() {
        assert_eq!(bar(0.5, 4, true), "##..");
        assert_eq!(bar(2.0, 3, true), "###");
    }
}
