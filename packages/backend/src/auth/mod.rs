mod middleware;
mod routes;
mod service;
mod token;

pub use middleware::{require_authenticated_user, AuthenticatedUser};
pub use routes::router;
pub use service::{AuthConfig, AuthService, RegisterRequest};

#[cfg(test)]
mod tests;
