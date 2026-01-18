use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HnItem {
    pub id: u64,
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub by: Option<String>,
    pub time: Option<u64>,
    pub text: Option<String>,
    pub url: Option<String>,
    pub score: Option<u32>,
    pub title: Option<String>,
    pub descendants: Option<u32>,
    #[serde(default)]
    pub kids: Vec<u64>,
    pub parent: Option<u64>,
    pub deleted: Option<bool>,
    pub dead: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Story {
    pub id: u64,
    pub title: String,
    pub url: Option<String>,
    pub score: u32,
    pub by: String,
    pub time: u64,
    pub descendants: u32,
    pub kids: Vec<u64>,
}

impl Story {
    pub fn from_item(item: HnItem) -> Option<Self> {
        Some(Story {
            id: item.id,
            title: item.title?,
            url: item.url,
            score: item.score.unwrap_or(0),
            by: item.by.unwrap_or_else(|| "[deleted]".to_string()),
            time: item.time.unwrap_or(0),
            descendants: item.descendants.unwrap_or(0),
            kids: item.kids,
        })
    }

    pub fn domain(&self) -> &str {
        self.url
            .as_ref()
            .and_then(|u| {
                u.split("://")
                    .nth(1)
                    .and_then(|s| s.split('/').next())
                    .map(|s| s.strip_prefix("www.").unwrap_or(s))
            })
            .unwrap_or("self")
    }

    /// URL to the HN discussion page for this story.
    pub fn hn_url(&self) -> String {
        format!("https://news.ycombinator.com/item?id={}", self.id)
    }

    /// URL to the story content (article URL, or HN page for self-posts).
    pub fn content_url(&self) -> String {
        self.url.clone().unwrap_or_else(|| self.hn_url())
    }
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub id: u64,
    pub text: String,
    pub by: String,
    pub time: u64,
    pub depth: usize,
    #[allow(dead_code)] // Kept for future nested threading
    pub kids: Vec<u64>,
}

impl Comment {
    pub fn from_item(item: HnItem, depth: usize) -> Option<Self> {
        if item.deleted.unwrap_or(false) || item.dead.unwrap_or(false) {
            return None;
        }

        Some(Comment {
            id: item.id,
            text: html_escape::decode_html_entities(&item.text?).to_string(),
            by: item.by.unwrap_or_else(|| "[deleted]".to_string()),
            time: item.time.unwrap_or(0),
            depth,
            kids: item.kids,
        })
    }

    /// URL to the HN permalink for this comment.
    pub fn hn_url(&self) -> String {
        format!("https://news.ycombinator.com/item?id={}", self.id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Feed {
    #[default]
    Top,
    New,
    Best,
    Ask,
    Show,
    Jobs,
}

impl Feed {
    pub fn endpoint(&self) -> &'static str {
        match self {
            Feed::Top => "topstories",
            Feed::New => "newstories",
            Feed::Best => "beststories",
            Feed::Ask => "askstories",
            Feed::Show => "showstories",
            Feed::Jobs => "jobstories",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Feed::Top => "Top",
            Feed::New => "New",
            Feed::Best => "Best",
            Feed::Ask => "Ask",
            Feed::Show => "Show",
            Feed::Jobs => "Jobs",
        }
    }

    pub fn all() -> &'static [Feed] {
        &[
            Feed::Top,
            Feed::New,
            Feed::Best,
            Feed::Ask,
            Feed::Show,
            Feed::Jobs,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(text: Option<&str>, deleted: bool, dead: bool) -> HnItem {
        HnItem {
            id: 1,
            item_type: Some("comment".to_string()),
            by: Some("testuser".to_string()),
            time: Some(1234567890),
            text: text.map(String::from),
            url: None,
            score: None,
            title: None,
            descendants: None,
            kids: vec![],
            parent: Some(0),
            deleted: if deleted { Some(true) } else { None },
            dead: if dead { Some(true) } else { None },
        }
    }

    #[test]
    fn test_story_domain_extraction() {
        let story = Story {
            id: 1,
            title: "Test".to_string(),
            url: Some("https://www.example.com/path".to_string()),
            score: 100,
            by: "user".to_string(),
            time: 0,
            descendants: 0,
            kids: vec![],
        };
        assert_eq!(story.domain(), "example.com");
    }

    #[test]
    fn test_story_self_domain() {
        let story = Story {
            id: 1,
            title: "Ask HN: Something".to_string(),
            url: None,
            score: 100,
            by: "user".to_string(),
            time: 0,
            descendants: 0,
            kids: vec![],
        };
        assert_eq!(story.domain(), "self");
    }

    #[test]
    fn test_feed_endpoints() {
        assert_eq!(Feed::Top.endpoint(), "topstories");
        assert_eq!(Feed::Ask.endpoint(), "askstories");
    }

    #[test]
    fn test_feed_cycling() {
        let feeds = Feed::all();
        assert_eq!(feeds[0], Feed::Top);
        assert_eq!(feeds[feeds.len() - 1], Feed::Jobs);

        // Test wraparound math (same logic as cycle_feed)
        let wrap = |idx: i32, len: i32| idx.rem_euclid(len) as usize;
        assert_eq!(wrap(-1, 6), 5); // Before first -> last
        assert_eq!(wrap(6, 6), 0); // After last -> first
    }

    #[test]
    fn test_comment_from_valid_item() {
        let item = make_item(Some("Hello world"), false, false);
        let comment = Comment::from_item(item, 2).unwrap();
        assert_eq!(comment.by, "testuser");
        assert_eq!(comment.depth, 2);
        assert!(comment.text.contains("Hello world"));
    }

    #[test]
    fn test_comment_skips_deleted() {
        let item = make_item(Some("Hello"), true, false);
        assert!(Comment::from_item(item, 0).is_none());
    }

    #[test]
    fn test_comment_skips_dead() {
        let item = make_item(Some("Hello"), false, true);
        assert!(Comment::from_item(item, 0).is_none());
    }

    #[test]
    fn test_comment_skips_empty_text() {
        let item = make_item(None, false, false);
        assert!(Comment::from_item(item, 0).is_none());
    }

    #[test]
    fn test_comment_decodes_html_entities() {
        let item = make_item(Some("&lt;script&gt; &amp; &quot;test&quot;"), false, false);
        let comment = Comment::from_item(item, 0).unwrap();
        assert_eq!(comment.text, "<script> & \"test\"");
    }
}
