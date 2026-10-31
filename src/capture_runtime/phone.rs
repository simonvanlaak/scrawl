//! PhoneSession's ordinary Drop is deliberately asynchronous for preview UI.
//! Recording owners must instead wait for helper teardown before releasing Engine.
use crate::phone::{PhoneRecording, PhoneSession};
use anyhow::Result;
use std::ops::Deref;

pub struct Phone(Option<PhoneSession>);
impl Phone {
    pub fn new(session: PhoneSession) -> Self {
        Self(Some(session))
    }
    pub fn stop(mut self) -> Result<PhoneRecording> {
        self.0.take().expect("live phone session").stop()
    }
}
impl Deref for Phone {
    type Target = PhoneSession;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("live phone session")
    }
}
impl Drop for Phone {
    fn drop(&mut self) {
        if let Some(session) = self.0.take() {
            let _ = session.stop();
        }
    }
}
