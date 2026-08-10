/// Strip `//` and `/* */` comments plus trailing commas so `serde_json` can
/// parse jsonc config files. String-aware; escapes respected.
pub fn to_json(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            match c {
                '\\' => {
                    if let Some(next) = chars.next() {
                        out.push(next);
                    }
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = ' ';
                for next in chars.by_ref() {
                    if prev == '*' && next == '/' {
                        break;
                    }
                    prev = next;
                }
            }
            ',' => {
                // Hold the comma until something other than whitespace or a
                // closing bracket decides whether it was trailing.
                let mut pending = String::from(",");
                let mut trailing = false;
                while let Some(&next) = chars.peek() {
                    if next.is_whitespace() {
                        pending.push(next);
                        chars.next();
                    } else if next == '}' || next == ']' {
                        trailing = true;
                        break;
                    } else {
                        break;
                    }
                }
                if trailing {
                    out.push_str(&pending[1..]);
                } else {
                    out.push_str(&pending);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_trailing_commas_are_removed() {
        let text = r#"{
  // line comment
  "a": 1, /* block
  comment */
  "b": [1, 2,],
}"#;
        let value: serde_json::Value = serde_json::from_str(&to_json(text)).unwrap();
        assert_eq!(value["a"], 1);
        assert_eq!(value["b"][1], 2);
    }

    #[test]
    fn strings_with_slashes_and_escapes_survive() {
        let text = r#"{"url": "https://x/y", "q": "a\"b, //not a comment"}"#;
        let value: serde_json::Value = serde_json::from_str(&to_json(text)).unwrap();
        assert_eq!(value["url"], "https://x/y");
        assert_eq!(value["q"], "a\"b, //not a comment");
    }
}
