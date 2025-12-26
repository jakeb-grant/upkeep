use std::process::Command;

/// A news item from the Arch Linux news feed
#[derive(Debug, Clone)]
pub struct NewsItem {
    pub title: String,
    pub link: String,
    pub description: String,
    pub author: String,
    pub pub_date: String,
    pub requires_attention: bool,
    pub related_packages: Vec<String>,
}

/// Detailed news info for the info pane
#[derive(Debug, Clone)]
pub struct NewsInfo {
    pub title: String,
    pub author: String,
    pub date: String,
    pub link: String,
    pub content: Vec<String>,
    pub related_packages: Vec<String>,
}

impl NewsItem {
    /// Convert to NewsInfo for the info pane
    pub fn to_info(&self) -> NewsInfo {
        let content: Vec<String> = self
            .description
            .lines()
            .map(|s| s.to_string())
            .collect();

        NewsInfo {
            title: self.title.clone(),
            author: self.author.clone(),
            date: self.pub_date.clone(),
            link: self.link.clone(),
            content,
            related_packages: self.related_packages.clone(),
        }
    }
}

/// Keywords that indicate manual intervention is required
const ATTENTION_KEYWORDS: &[&str] = &[
    "manual intervention",
    "action required",
    "immediately",
    "before upgrading",
    "require manual",
    "must be",
    "breaking change",
];

/// Fetch and parse news from Arch Linux RSS feed
pub fn fetch_news(installed_packages: &[String]) -> Result<Vec<NewsItem>, String> {
    let output = Command::new("curl")
        .args(["-s", "-m", "10", "https://archlinux.org/feeds/news/"])
        .output()
        .map_err(|e| format!("Failed to run curl: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "curl failed with status: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    let xml = String::from_utf8_lossy(&output.stdout);
    parse_rss_feed(&xml, installed_packages)
}

/// Parse RSS feed XML using the rss crate
fn parse_rss_feed(xml: &str, installed_packages: &[String]) -> Result<Vec<NewsItem>, String> {
    let channel = xml
        .parse::<rss::Channel>()
        .map_err(|e| format!("Failed to parse RSS: {}", e))?;

    let items = channel
        .items()
        .iter()
        .map(|item| {
            let title = item.title().unwrap_or("").to_string();
            let link = item.link().unwrap_or("").to_string();
            let raw_description = item.description().unwrap_or("");
            let description = strip_html(raw_description);
            let author = item
                .dublin_core_ext()
                .and_then(|dc| dc.creators().first().map(|s| s.as_str()))
                .unwrap_or("")
                .to_string();
            let pub_date = format_pub_date(item.pub_date().unwrap_or(""));

            let full_text = format!("{} {}", title, description);
            let requires_attention = check_requires_attention(&full_text);
            let related_packages = find_related_packages(&full_text, installed_packages);

            NewsItem {
                title,
                link,
                description,
                author,
                pub_date,
                requires_attention,
                related_packages,
            }
        })
        .collect();

    Ok(items)
}

/// Check if text contains keywords indicating manual intervention is required
fn check_requires_attention(text: &str) -> bool {
    let lower = text.to_lowercase();
    ATTENTION_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

/// Find installed packages mentioned in the text
/// Matches both exact names and base name variants (e.g., "grub" matches "grub-btrfs")
pub fn find_related_packages(text: &str, installed_packages: &[String]) -> Vec<String> {
    let lower = text.to_lowercase();

    installed_packages
        .iter()
        .filter(|pkg| {
            if pkg.len() < 3 {
                return false;
            }

            let pkg_lower = pkg.to_lowercase();

            // Check exact package name
            if word_in_text(&lower, &pkg_lower) {
                return true;
            }

            // Check base name variants
            // e.g., if we have "grub-btrfs" installed and news mentions "grub"
            let parts: Vec<&str> = pkg_lower.split('-').collect();
            for i in 1..parts.len() {
                let base = parts[..i].join("-");
                if base.len() >= 3 && word_in_text(&lower, &base) {
                    return true;
                }
            }

            false
        })
        .cloned()
        .collect()
}

/// Check if a word appears in text with word boundaries
/// Avoids partial matches like "go" in "google"
fn word_in_text(text: &str, word: &str) -> bool {
    for (i, _) in text.match_indices(word) {
        // Check character before match
        let before_ok = i == 0
            || text[..i]
                .chars()
                .last()
                .map(|c| !c.is_alphanumeric())
                .unwrap_or(true);

        // Check character after match
        let after_ok = i + word.len() >= text.len()
            || text[i + word.len()..]
                .chars()
                .next()
                .map(|c| !c.is_alphanumeric() && c != '-')
                .unwrap_or(true);

        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Strip HTML tags and decode entities
fn strip_html(html: &str) -> String {
    let mut result = html.to_string();

    // Convert block elements to newlines (before removing tags)
    result = result.replace("<p>", "\n");
    result = result.replace("</p>", "\n");
    result = result.replace("<br>", "\n");
    result = result.replace("<br/>", "\n");
    result = result.replace("<br />", "\n");
    result = result.replace("<li>", "\n- ");
    result = result.replace("</li>", "");
    result = result.replace("<ul>", "\n");
    result = result.replace("</ul>", "\n");
    result = result.replace("<ol>", "\n");
    result = result.replace("</ol>", "\n");
    result = result.replace("<pre>", "\n");
    result = result.replace("</pre>", "\n");
    result = result.replace("<code>", "");
    result = result.replace("</code>", "");

    // Remove all remaining HTML tags
    let mut in_tag = false;
    let mut cleaned = String::new();
    for ch in result.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => cleaned.push(ch),
            _ => {}
        }
    }

    // Decode HTML entities after removing tags
    let result = decode_html_entities(&cleaned);

    // Clean up whitespace
    let lines: Vec<&str> = result
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    lines.join("\n")
}

/// Decode HTML entities (named and numeric)
fn decode_html_entities(text: &str) -> String {
    let mut result = text.to_string();

    // Named entities (most common first for efficiency)
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&apos;", "'");
    result = result.replace("&nbsp;", " ");

    // Typographic entities
    result = result.replace("&mdash;", "\u{2014}"); // —
    result = result.replace("&ndash;", "\u{2013}"); // –
    result = result.replace("&hellip;", "\u{2026}"); // …
    result = result.replace("&lsquo;", "\u{2018}"); // '
    result = result.replace("&rsquo;", "\u{2019}"); // '
    result = result.replace("&ldquo;", "\u{201C}"); // "
    result = result.replace("&rdquo;", "\u{201D}"); // "
    result = result.replace("&laquo;", "\u{00AB}"); // «
    result = result.replace("&raquo;", "\u{00BB}"); // »
    result = result.replace("&bull;", "\u{2022}"); // •

    // Copyright/trademark
    result = result.replace("&copy;", "©");
    result = result.replace("&reg;", "®");
    result = result.replace("&trade;", "™");

    // Decode numeric character references (&#39; &#x27; etc.)
    result = decode_numeric_entities(&result);

    result
}

/// Decode numeric HTML entities like &#39; and &#x27;
fn decode_numeric_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' && chars.peek() == Some(&'#') {
            chars.next(); // consume '#'
            let mut num_str = String::new();
            let is_hex = chars.peek() == Some(&'x') || chars.peek() == Some(&'X');
            if is_hex {
                chars.next(); // consume 'x' or 'X'
            }

            // Collect digits
            while let Some(&c) = chars.peek() {
                if c == ';' {
                    chars.next();
                    break;
                }
                if (is_hex && c.is_ascii_hexdigit()) || (!is_hex && c.is_ascii_digit()) {
                    num_str.push(c);
                    chars.next();
                } else {
                    break;
                }
            }

            // Try to parse and convert to char
            let codepoint = if is_hex {
                u32::from_str_radix(&num_str, 16).ok()
            } else {
                num_str.parse::<u32>().ok()
            };

            if let Some(cp) = codepoint {
                if let Some(decoded) = char::from_u32(cp) {
                    result.push(decoded);
                    continue;
                }
            }

            // Failed to decode - output original sequence
            result.push('&');
            result.push('#');
            if is_hex {
                result.push('x');
            }
            result.push_str(&num_str);
        } else {
            result.push(ch);
        }
    }

    result
}

/// Format RFC 2822 date to a more readable format
fn format_pub_date(date: &str) -> String {
    // Input: "Fri, 20 Dec 2024 00:00:00 +0000"
    // Output: "Dec 20, 2024"
    let parts: Vec<&str> = date.split_whitespace().collect();
    if parts.len() >= 4 {
        format!("{} {}, {}", parts[2], parts[1], parts[3])
    } else {
        date.to_string()
    }
}

/// Format date for short display in list
pub fn format_short_date(date: &str) -> String {
    // Input: "Dec 20, 2024"
    // Output: "Dec 20"
    let parts: Vec<&str> = date.split(',').collect();
    if !parts.is_empty() {
        parts[0].to_string()
    } else {
        date.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_html() {
        let html = "<p>Hello <strong>world</strong>!</p>";
        let result = strip_html(html);
        assert_eq!(result, "Hello world!");
    }

    #[test]
    fn test_check_requires_attention() {
        assert!(check_requires_attention("Manual intervention required"));
        assert!(check_requires_attention("Please do this immediately"));
        assert!(!check_requires_attention("Regular update available"));
    }

    #[test]
    fn test_find_related_packages() {
        let installed = vec!["grub".to_string(), "linux".to_string(), "go".to_string()];
        let text = "The grub package needs updating";
        let related = find_related_packages(text, &installed);
        assert!(related.contains(&"grub".to_string()));
        assert!(!related.contains(&"go".to_string())); // Too short
    }

    #[test]
    fn test_find_related_packages_variants() {
        let installed = vec![
            "grub".to_string(),
            "grub-btrfs".to_string(),
            "python-numpy".to_string(),
        ];
        // News mentions "grub" - should match both grub and grub-btrfs
        let text = "Users of grub need to regenerate config";
        let related = find_related_packages(text, &installed);
        assert!(related.contains(&"grub".to_string()));
        assert!(related.contains(&"grub-btrfs".to_string()));
        assert!(!related.contains(&"python-numpy".to_string()));
    }

    #[test]
    fn test_word_boundaries() {
        let installed = vec!["mesa".to_string(), "lib".to_string()];
        // "mesa" should match, "lib" should not (too short)
        let text = "Update your mesa drivers";
        let related = find_related_packages(text, &installed);
        assert!(related.contains(&"mesa".to_string()));

        // Should not match "mesa" inside "gamescope"
        let text2 = "gamescope update available";
        let related2 = find_related_packages(text2, &installed);
        assert!(!related2.contains(&"mesa".to_string()));
    }

    #[test]
    fn test_format_pub_date() {
        let date = "Fri, 20 Dec 2024 00:00:00 +0000";
        assert_eq!(format_pub_date(date), "Dec 20, 2024");
    }

    #[test]
    fn test_html_entity_decoding() {
        // Basic entities
        assert_eq!(
            strip_html("&lt;script&gt; &amp; &quot;test&quot;"),
            "<script> & \"test\""
        );

        // Typographic entities
        assert_eq!(strip_html("test&mdash;value"), "test\u{2014}value");
        assert_eq!(strip_html("a&ndash;b"), "a\u{2013}b");
        assert_eq!(strip_html("wait&hellip;"), "wait\u{2026}");

        // Quotes
        assert_eq!(
            strip_html("&ldquo;quoted&rdquo;"),
            "\u{201C}quoted\u{201D}"
        );
        assert_eq!(
            strip_html("&lsquo;single&rsquo;"),
            "\u{2018}single\u{2019}"
        );
    }

    #[test]
    fn test_numeric_entity_decoding() {
        // Decimal numeric entities
        assert_eq!(strip_html("&#39;"), "'");
        assert_eq!(strip_html("&#34;"), "\"");
        assert_eq!(strip_html("&#169;"), "©");

        // Hex numeric entities
        assert_eq!(strip_html("&#x27;"), "'");
        assert_eq!(strip_html("&#x22;"), "\"");
        assert_eq!(strip_html("&#xA9;"), "©");

        // Mixed content
        assert_eq!(
            strip_html("It&#39;s &ldquo;great&rdquo;"),
            "It's \u{201C}great\u{201D}"
        );
    }
}
