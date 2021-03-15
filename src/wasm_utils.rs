use bevy::log::{Level, LogSettings};

/// Get logging config for WASM
#[cfg(wasm)]
pub fn get_log_config() -> LogSettings {
    // Check for RUST_LOG query string to get log level
    let level = (|| -> Option<Level> {
        let query_string: String = web_sys::window()?.location().search().ok()?;

        parse_url_query_string(&query_string, "RUST_LOG")
            .map(|x| x.parse().ok())
            .flatten()
    })();

    // Set log level
    LogSettings {
        level: level.unwrap_or(Level::INFO),
        ..Default::default()
    }
}

#[cfg(any(target_arch = "wasm32", test))]
/// Parse the query string as returned by `web_sys::window()?.location().search()?` and get a
/// specific key out of it.
pub fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix("?")?;

    for pair in query_string.split("&") {
        let mut pair = pair.split("=");
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_url_query_string() {
        assert_eq!(
            Some("info"),
            parse_url_query_string("?RUST_LOG=info", "RUST_LOG")
        );
        assert_eq!(
            Some("debug"),
            parse_url_query_string("?RUST_LOG=debug&hello=world&foo=bar", "RUST_LOG")
        );
        assert_eq!(
            Some("trace"),
            parse_url_query_string("?hello=world&RUST_LOG=trace&foo=bar", "RUST_LOG")
        );
        assert_eq!(
            None,
            parse_url_query_string("?hello=world&foo=bar", "RUST_LOG")
        );
    }
}
