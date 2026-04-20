use crate::db::User;



pub enum SessionState {
    LongToken,
    PPauthToken,
    PrivateAuthToken
}

pub struct Session {
    state: SessionState,
    user: Option<User>,
}
