mod parse;
mod types;

use failure::Error as DynErr;

const USER_AGENT: &'static str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:69.0) Gecko/20100101 Firefox/69.0";

pub use types::{Comment, Post, VoteAction};

/// Unauthenticated Hackernews client.
///
/// See [AuthenticatedClient] for authenticated actions.
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("user-agent", USER_AGENT.parse().unwrap());
        let inner = reqwest::Client::builder()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .unwrap();
        Self { inner }
    }

    fn get_html(&self, path: &str) -> Result<String, reqwest::Error> {
        let url = format!("https://news.ycombinator.com/{}", path);
        self.inner.get(&url).send()?.error_for_status()?.text()
    }

    fn get_dom(&self, path: &str) -> Result<scraper::Html, DynErr> {
        let html = self.get_html(path)?;
        Ok(scraper::Html::parse_document(&html))
    }

    /// Get the current top posts.
    pub fn top(&self, page: u64) -> Result<Vec<Post>, DynErr> {
        let doc = self.get_dom(&format!("news?p={}", page))?;
        parse::parse_list(doc).map_err(Into::into)
    }

    /// Get a single post with comments.
    pub fn submission(&self, id: &str) -> Result<Post, DynErr> {
        let url = format!("item?id={}", id);
        let dom = self.get_dom(&url)?;
        parse::parse_submission(id.to_string(), dom).map_err(Into::into)
    }
}

pub struct AuthenticatedClient {
    client: Client,
}

impl std::ops::Deref for AuthenticatedClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl AuthenticatedClient {
    /// Log in.
    pub fn login(username: &str, password: &str) -> Result<Self, DynErr> {
        let inner = reqwest::Client::builder().cookie_store(true).build()?;

        let _login_page = inner
            .get("https://news.ycombinator.com/login?goto=news")
            .send()?
            .error_for_status()?
            .text()?;

        let res = inner
            .post("https://news.ycombinator.com/login")
            .form(&serde_json::json!({
                "goto": "news",
                "acct": username,
                "pw": password,
            }))
            .send()?
            .error_for_status()?;

        if res.url().as_str() != "https://news.ycombinator.com/news" {
            // TODO: parse error message.
            return Err(failure::format_err!(
                "Login failued: invalid credentials?"
            ));
        }

        Ok(Self {
            client: Client { inner },
        })
    }

    /// Create a new account.
    pub fn signup(username: &str, password: &str) -> Result<Self, DynErr> {
        let inner = reqwest::Client::builder().cookie_store(true).build()?;

        let _login_page = inner
            .get("https://news.ycombinator.com/login?goto=news")
            .send()?
            .error_for_status()?
            .text()?;

        let res = inner
            .post("https://news.ycombinator.com/login")
            .form(&serde_json::json!({
                "goto": "news",
                "creating": "t",
                "acct": username,
                "pw": password,
            }))
            .send()?
            .error_for_status()?;

        if res.url().as_str() != "https://news.ycombinator.com/news" {
            // TODO: parse error message.
            return Err(failure::format_err!("Signup failed"));
        }

        Ok(Self {
            client: Client { inner },
        })
    }

    /// Up or downvote a post or comment.
    ///
    /// a [VoteAction] can be retrieved from the [Post] and [Post] types.
    pub fn vote(&self, action: &VoteAction) -> Result<(), reqwest::Error> {
        let url = format!("https://news.ycombinator.com/{}", action.url());
        self.client.inner.get(&url).send()?.error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_top() {
        let c = Client::new();
        let items = c.top(1).unwrap();
        assert!(items.len() >= 20);
    }

    #[test]
    fn test_submission() {
        let c = Client::new();
        let s = c.submission("20993456").unwrap();

        assert_eq!(
            s.title,
            "Where you are born is more predictive of your future than any other factor"
        );
        assert!(s.score > 150);
    }

    #[test]
    fn test_auth() {
        let creds = std::env::var("HN_CREDENTIALS")
            .expect("Could not run login test: HN_CREDENTIALS env var not set");
        let mut parts = creds.split(':');
        let (user, pw) = (parts.next().unwrap(), parts.next().unwrap());

        let c = AuthenticatedClient::login(user, pw).unwrap();
        let items = c.top(0).unwrap();

        let item = items
            .iter()
            .find(|item| {
                item.vote.as_ref().map(|v| v.is_upvote()).unwrap_or(false)
            })
            .unwrap();

        // Upvote.
        c.vote(item.vote.as_ref().unwrap()).unwrap();

        let sub = c.submission(&item.id).unwrap();
        let down = sub.vote.as_ref().unwrap();
        assert_eq!(down.is_upvote(), false);

        c.vote(down).unwrap();

        let sub = c.submission(&item.id).unwrap();
        let up = sub.vote.as_ref().unwrap();
        assert_eq!(up.is_upvote(), true);
    }
}
