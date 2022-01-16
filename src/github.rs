use futures_executor::block_on;
use octocrab::{models::activity::Notification, Octocrab, Page};

use crate::error::{Error, Result};

pub struct GitHub {
    octocrab: Octocrab,
    notif_cache: Option<Page<Notification>>,
}

impl GitHub {
    fn new(token: &str) -> Result<Self> {
        Ok(Self {
            octocrab: Octocrab::builder()
                .personal_token(token.to_string())
                .build()?,
            notif_cache: None,
        })
    }

    pub fn token_from_env() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN").map_err(|_| Error::AuthError)?;
        Self::new(&token)
    }

    pub fn notifications(&mut self, reload: bool) -> Result<&Page<Notification>> {
        if self.notif_cache.is_none() || reload {
            let notifs = block_on(self.octocrab.activity().notifications().list().send())
                .map_err(|e: octocrab::Error| Error::from(e))?;
            self.notif_cache = Some(notifs);
        }
        return Ok(self.notif_cache.as_ref().unwrap());
    }
}
