#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoteAction {
    Upvote(String),
    Downvote(String),
}

impl VoteAction {
    pub(crate) fn url(&self) -> &str {
        match self {
            Self::Upvote(ref url) => &url,
            Self::Downvote(ref url) => &url,
        }
    }

    pub fn is_upvote(&self) -> bool {
        match self {
            Self::Upvote(_) => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub url: String,
    pub username: String,
    pub score: u64,
    pub comment_count: u64,
    pub comments: Vec<Comment>,

    pub vote: Option<VoteAction>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub id: String,
    pub depth: u32,
    pub age: String,
    pub username: String,
    pub content_html: String,
    pub children: Vec<Comment>,

    pub upvote: Option<VoteAction>,
    pub downvote: Option<VoteAction>,
}
