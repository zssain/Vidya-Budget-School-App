//! Authentication: password hashing (P2.5), the in-memory session store, and
//! `AuthService` — sign-in, first password, change password, sign-out, language
//! and actor resolution (P2.6).

pub mod password;
pub mod service;
pub mod sessions;

pub use service::{AuthService, CurrentUserDto, SignInResult};
pub use sessions::SessionStore;
