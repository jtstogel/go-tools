use std::collections::HashMap;

/// A view into the text of a config.
pub struct ConfigView<'a> {
    data: HashMap<&'a str, &'a str>,
}

impl<'a> ConfigView<'a> {
    pub fn parse(config: &'a str) -> Self {
        // Finds lines of the form:
        // myKey = myValue  # Optional comment
        let kv_regex = lazy_regex::regex!("^(\\w+)\\s*=\\s*(\\w+)\\s*(?:#.*)?$");
        let data = config
            .lines()
            .filter_map(|line| {
                let captures = kv_regex.captures(line)?;
                let key = captures.get(1)?;
                let value = captures.get(2)?;
                Some((key.as_str(), value.as_str()))
            })
            .collect();
        Self { data }
    }

    pub fn get(&self, key: &str) -> Option<&'a str> {
        self.data.get(key).copied()
    }
}

#[cfg(test)]
mod test {
    use crate::config::ConfigView;

    #[test]
    fn test_basic() {
        let view = ConfigView::parse("key=value");
        assert_eq!(view.get("key"), Some("value"));
    }

    #[test]
    fn test_multiple_values() {
        let view = ConfigView::parse(
            "
key1=value1
key2=value2
",
        );
        assert_eq!(view.get("key1"), Some("value1"));
        assert_eq!(view.get("key2"), Some("value2"));
    }

    #[test]
    fn test_whitespace() {
        let view = ConfigView::parse("key	=  value");
        assert_eq!(view.get("key"), Some("value"));
        assert_eq!(view.get("key"), Some(""));

        assert_eq!(1, 1);
    }

    #[test]
    fn test_comment() {
        let view = ConfigView::parse("key = value # comment");
        assert_eq!(view.get("key"), Some("value"));
    }

    #[test]
    fn test_ignores_comment_line() {
        let view = ConfigView::parse(
            "
# comment
key1 = value1

# comment
key2 = value2
",
        );
        assert_eq!(view.get("key1"), Some("value1"));
        assert_eq!(view.get("key2"), Some("value2"));
    }
}
