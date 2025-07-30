use std::cmp::Reverse;
use std::fmt::Write;

use fxhash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{Html, Resolver};

/// Build HTML detailing the contributors between two tags.
pub fn contributors(resolver: &dyn Resolver, from: &str, to: &str) -> Option<Html> {
    let staff = ["laurmaedje", "reknih"];
    let bots = ["dependabot[bot]"];

    // Determine number of contributions per person.
    let mut contributors = FxHashMap::<String, Contributor>::default();
    for commit in resolver.commits(from, to) {
        contributors
            .entry(commit.author.login.clone())
            .or_insert_with(|| Contributor {
                login: commit.author.login,
                avatar: commit.author.avatar_url,
                contributions: 0,
            })
            .contributions += 1;
    }

    // Keep only non-staff people.
    let mut contributors: Vec<_> = contributors
        .into_values()
        .filter(|c| {
            let login = c.login.as_str();
            !staff.contains(&login) && !bots.contains(&login)
        })
        .collect();

    // Sort by highest number of commits.
    contributors.sort_by_key(|c| (Reverse(c.contributions), c.login.clone()));
    if contributors.is_empty() {
        return None;
    }

    let mut html = "Thanks to everyone who contributed to this release!".to_string();
    html += "<ul class=\"contribs\">";

    for Contributor { login, avatar, contributions } in contributors {
        let login = login.replace('\"', "&quot;").replace('&', "&amp;");
        let avatar = avatar.replace("?v=", "?s=64&v=");
        let s = if contributions > 1 { "s" } else { "" };
        write!(
            html,
            r#"<li>
              <a href="https://github.com/{login}" target="_blank">
                <img
                  width="64"
                  height="64"
                  src="{avatar}"
                  alt="GitHub avatar of {login}"
                  title="@{login} made {contributions} contribution{s}"
                  crossorigin="anonymous"
                >
              </a>
            </li>"#
        )
        .unwrap();
    }

    html += "</ul>";

    Some(Html::new(html))
}

#[derive(Debug)]
struct Contributor {
    login: String,
    avatar: String,
    contributions: usize,
}

/// A commit on the `typst` repository.
#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    author: Author,
}

/// A commit author.
#[derive(Debug, Serialize, Deserialize)]
pub struct Author {
    login: String,
    avatar_url: String,
}
