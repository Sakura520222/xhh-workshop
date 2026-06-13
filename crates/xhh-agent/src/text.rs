pub(crate) fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_chars;

    #[test]
    fn truncates_by_character_count() {
        assert_eq!(truncate_chars("测试文本", 2), "测试...");
        assert_eq!(truncate_chars("short", 5), "short");
    }
}
