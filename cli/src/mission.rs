/// Tokenize Valve KeyValues text into a flat list of tokens.
/// Handles quoted strings, braces, and // comments.
#[derive(Debug, PartialEq)]
enum Token {
    String(String),
    OpenBrace,
    CloseBrace,
}

fn tokenize(raw: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = raw.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '/' => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\n' {
                            break;
                        }
                    }
                }
            }
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '"' {
                        break;
                    }
                    s.push(c);
                }
                tokens.push(Token::String(s));
            }
            '{' => {
                chars.next();
                tokens.push(Token::OpenBrace);
            }
            '}' => {
                chars.next();
                tokens.push(Token::CloseBrace);
            }
            _ if ch.is_whitespace() => {
                chars.next();
            }
            _ => {
                chars.next();
            }
        }
    }

    tokens
}

fn expect_brace(tokens: &[Token], pos: &mut usize, open: bool) -> bool {
    let expected = if open {
        Token::OpenBrace
    } else {
        Token::CloseBrace
    };
    if *pos < tokens.len() && tokens[*pos] == expected {
        *pos += 1;
        true
    } else {
        false
    }
}

/// Extract ALL Map values under modes -> coop from Valve KeyValues text.
/// Returns all Map values found (in order), or empty vec if none.
pub fn parse_coop_maps(raw: &str) -> Vec<String> {
    let tokens = tokenize(raw);
    let mut pos = 0;

    // Find "modes"
    while pos < tokens.len() {
        if let Token::String(s) = &tokens[pos] {
            if s == "modes" {
                pos += 1;
                break;
            }
        }
        pos += 1;
    }
    if pos >= tokens.len() {
        return vec![];
    }

    // Expect '{'
    if !expect_brace(&tokens, &mut pos, true) {
        return vec![];
    }

    // Search inside modes block for "coop"
    let mut depth: u32 = 1;
    while depth > 0 && pos < tokens.len() {
        match &tokens[pos] {
            Token::OpenBrace => depth += 1,
            Token::CloseBrace => depth -= 1,
            Token::String(s) if s == "coop" && depth == 1 => {
                pos += 1;
                if !expect_brace(&tokens, &mut pos, true) {
                    return vec![];
                }

                // Collect ALL Map values inside coop block
                return collect_map_values(&tokens, &mut pos);
            }
            _ => {}
        }
        pos += 1;
    }

    vec![]
}

/// Collect all "Map" values at any depth inside the current block.
/// Caller has already consumed the opening '{', we start at depth 1.
fn collect_map_values(tokens: &[Token], pos: &mut usize) -> Vec<String> {
    let mut maps = Vec::new();
    let mut depth: u32 = 1;
    while depth > 0 && *pos < tokens.len() {
        match &tokens[*pos] {
            Token::OpenBrace => depth += 1,
            Token::CloseBrace => depth -= 1,
            Token::String(s) if s == "Map" => {
                *pos += 1;
                if *pos < tokens.len() {
                    if let Token::String(val) = &tokens[*pos] {
                        maps.push(val.clone());
                    }
                }
            }
            _ => {}
        }
        *pos += 1;
    }
    maps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coop_maps_basic() {
        let input = r#"
"modes"
{
    "coop"
    {
        "Map" "c1m1_hotel"
    }
}
"#;
        assert_eq!(parse_coop_maps(input), vec!["c1m1_hotel"]);
    }

    #[test]
    fn parse_coop_maps_multiple_chapters() {
        let input = r#"
"modes"
{
    "coop"
    {
        "1"
        {
            "Map" "p84m1_apartment"
        }
        "2"
        {
            "Map" "p84m2_eltrain"
        }
        "3"
        {
            "Map" "p84m3_tunnel"
        }
    }
}
"#;
        assert_eq!(
            parse_coop_maps(input),
            vec!["p84m1_apartment", "p84m2_eltrain", "p84m3_tunnel"]
        );
    }

    #[test]
    fn parse_coop_maps_with_comments() {
        let input = r#"
// Header comment
"modes"
{
    "coop"  // cooperative mode
    {
        "Map" "c2m1_highway"  // first map
    }
}
"#;
        assert_eq!(parse_coop_maps(input), vec!["c2m1_highway"]);
    }

    #[test]
    fn parse_coop_maps_sibling_keys_ignored() {
        let input = r#"
"modes"
{
    "versus"
    {
        "Map" "c5m1_waterfront"
    }
    "coop"
    {
        "Map" "c1m1_hotel"
    }
}
"#;
        assert_eq!(parse_coop_maps(input), vec!["c1m1_hotel"]);
    }

    #[test]
    fn parse_coop_maps_missing() {
        let input = r#"
"modes"
{
    "versus"
    {
        "Map" "c5m1_waterfront"
    }
}
"#;
        assert_eq!(parse_coop_maps(input), Vec::<String>::new());
    }

    #[test]
    fn parse_coop_maps_empty() {
        assert_eq!(parse_coop_maps(""), Vec::<String>::new());
    }

    #[test]
    fn parse_coop_maps_no_modes() {
        assert_eq!(
            parse_coop_maps(r#""something" { "key" "val" }"#),
            Vec::<String>::new()
        );
    }

    #[test]
    fn parse_coop_maps_tabs_instead_of_spaces() {
        let input = "\"modes\"\n{\n\t\"coop\"\n\t{\n\t\t\"Map\"\t\"c1m1_hotel\"\n\t}\n}";
        assert_eq!(parse_coop_maps(input), vec!["c1m1_hotel"]);
    }
}
