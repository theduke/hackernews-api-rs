use scraper::{ElementRef, Html as Document, Selector};

use super::types::{Comment, Post, VoteAction};

#[derive(Debug)]
pub struct ParseError {
    message: String,
}

impl ParseError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

fn sel(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}

fn el_text(el: &ElementRef) -> String {
    el.text()
        .fold(String::new(), |mut s, t| {
            let clean = t.trim();
            if !clean.is_empty() {
                s.push(' ');
                s.push_str(&clean);
            }
            s
        })
        .trim()
        .to_string()
}

fn el_text_opt(el: &ElementRef) -> Option<String> {
    let txt = el_text(el);
    if txt.is_empty() {
        None
    } else {
        Some(txt)
    }
}

fn parse_username(el: ElementRef) -> Result<String, ParseError> {
    el.select(&sel(".hnuser"))
        .next()
        .and_then(|el| el_text_opt(&el))
        .ok_or_else(|| ParseError::new("Could not find username"))
}

type Url = String;
type Title = String;

fn parse_storylink(el: ElementRef) -> Result<(Title, Url), ParseError> {
    let storylink = el
        .select(&sel(".storylink"))
        .next()
        .ok_or_else(|| ParseError::new("Could not find story link"))?;

    let url = storylink
        .value()
        .attr("href")
        .ok_or_else(|| ParseError::new("Story link has no href"))?
        .to_string();

    let title = el_text_opt(&storylink)
        .ok_or_else(|| ParseError::new("Could not find title"))?;

    Ok((title, url))
}

fn parse_score(el: ElementRef) -> Result<u64, ParseError> {
    el.select(&sel(".score"))
        .next()
        .and_then(|el| {
            el_text(&el)
                .split(' ')
                .next()
                .and_then(|raw| raw.parse::<u64>().ok())
        })
        .ok_or_else(|| ParseError::new("Could not find score"))
}

fn parse_comment_count(el: ElementRef) -> Result<u64, ParseError> {
    let text = el
        .select(&sel("a"))
        .map(|a| el_text(&a))
        .filter(|txt| txt.ends_with("comments") || txt == "discuss")
        .last()
        .ok_or_else(|| ParseError::new("Could not find comment count"))?;

    if text == "discuss" {
        Ok(0)
    } else {
        text.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .map_err(|e| {
                ParseError::new(format!("Could not parse comment count: {}", e))
            })
    }
}

fn parse_upvote(el: ElementRef) -> Option<VoteAction> {
    let a = el
        .select(&sel("a"))
        .find(|el| el.value().attr("href").unwrap_or("").contains("how=up"))
        .filter(|el| {
            !el.value().attr("class").unwrap_or("").contains("nosee")
        })?;

    let url = a.value().attr("href").unwrap().to_string();
    Some(VoteAction::Upvote(url))
}

fn parse_downvote(el: ElementRef) -> Option<VoteAction> {
    let a = el
        .select(&sel("a"))
        .find(|el| el.value().attr("href").unwrap_or("").contains("how=un"))?;
    let url = a.value().attr("href").unwrap().to_string();
    Some(VoteAction::Downvote(url))
}

pub fn parse_list(doc: Document) -> Result<Vec<Post>, ParseError> {
    doc.select(&sel(".athing"))
        .map(|row_ref| -> Result<_, _> {
            let row = row_ref.value();

            let id = row
                .attr("id")
                .ok_or_else(|| {
                    ParseError::new("Could not get id for submission")
                })?
                .to_string();

            let (title, url) = parse_storylink(row_ref)?;

            let action_row_ref = row_ref
                .next_sibling()
                .and_then(|node| ElementRef::wrap(node))
                .ok_or_else(|| ParseError::new("Could not find action row"))?;

            let upvote = parse_upvote(row_ref);
            let downvote = parse_downvote(action_row_ref);
            let vote = upvote.or(downvote);

            let comment_count =
                parse_comment_count(action_row_ref).unwrap_or(0);
            let score = parse_score(action_row_ref).unwrap_or(0);
            let username = parse_username(action_row_ref)
                .unwrap_or("<unknown>".to_string());

            Ok(Post {
                id,
                title,
                username,
                url,
                score,
                comment_count,
                comments: Vec::new(),
                vote,
            })
        })
        .collect()
}

fn parse_comment(el: ElementRef) -> Result<Comment, ParseError> {
    let username = parse_username(el)?;

    let id = el
        .value()
        .attr("id")
        .ok_or_else(|| ParseError::new("Could not determine comment id"))?
        .to_string();

    let depth = el
        .select(&sel(".ind img"))
        .next()
        .and_then(|el| el.value().attr("width"))
        .and_then(|width| width.parse::<u32>().ok())
        .map(|width| width / 40)
        .ok_or_else(|| ParseError::new("Could not determine comment depth"))?;

    let age = el
        .select(&sel(".age"))
        .next()
        .and_then(|el| el_text_opt(&el))
        .ok_or_else(|| ParseError::new("Could not find comment age"))?;

    let content_html =
        el.select(&sel(".comment"))
            .next()
            .map(|el| el.html())
            .ok_or_else(|| ParseError::new("Could not find comment text"))?;

    let (upvote, downvote) = el
        .select(&sel(".votelinks"))
        .next()
        .map(|el| {
            let mut up = None;
            let mut down = None;
            for link in el.select(&sel("a")) {
                if let Some(href) = link.value().attr("href") {
                    if href.contains("how=up") {
                        up = Some(VoteAction::Upvote(href.to_string()));
                    } else if href.contains("how=un") {
                        down = Some(VoteAction::Downvote(href.to_string()));
                    }
                }
            }
            (up, down)
        })
        .unwrap_or((None, None));

    Ok(Comment {
        id,
        depth,
        age,
        username,
        content_html,
        children: Vec::new(),
        upvote,
        downvote,
    })
}

pub fn parse_submission(id: String, dom: Document) -> Result<Post, ParseError> {
    let header = dom
        .select(&sel(".fatitem"))
        .next()
        .ok_or_else(|| ParseError::new("Could not find post header"))?;

    let (title, url) = parse_storylink(header)?;
    let username = parse_username(header)?;
    let score = parse_score(header)?;

    let upvote = parse_upvote(header);
    let downvote = parse_downvote(header);
    let vote = upvote.or(downvote);
    let comment_count = parse_comment_count(header)?;

    let comments = dom
        .select(&sel(".comment-tree .athing.comtr"))
        .map(parse_comment)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Post {
        id,
        title,
        url,
        username,
        score,
        comment_count,
        comments,
        vote,
    })
}
