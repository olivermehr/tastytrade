use pretty_simple_display::{DebugPretty, DisplaySimple};
use serde::{Deserialize, Serialize};

/// Login credentials for authentication.
///
/// This struct holds the login information required for authentication, including
/// the username, password, and a "remember me" flag.  It's designed for
/// serialization with kebab-case renaming for compatibility with external APIs.
#[derive(DebugPretty, DisplaySimple, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoginCredentials {
    /// The grant type for login.
    pub grant_type: String,
    /// The client secret for login.
    pub client_secret: String,
    /// The refresh token for login.
    pub refresh_token: String,
}

#[allow(dead_code)]
#[derive(DebugPretty, DisplaySimple, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
/// Represents a user in a login response.  This struct is used for deserializing the JSON response
/// received after a successful login.  The `#[serde(rename_all = "kebab-case")]` attribute ensures
/// that the fields in the JSON response are matched to the struct fields correctly, even if the
/// casing is different (e.g., "external-id" in JSON will map to `external_id` in the struct).
pub struct LoginResponseUser {
    /// The user's email address.
    pub email: String,
    /// The user's username.
    pub username: String,
    /// The user's external ID.
    pub external_id: String,
}

/// Represents the response received after a successful login.
///
/// This struct is used for deserializing the JSON response.
/// The `#[serde(rename_all = "kebab-case")]` attribute ensures that the
/// fields in the JSON response are matched to the struct fields correctly,
/// even if the casing is different (e.g., "session-token" in JSON will map to
/// `session_token` in the struct).
#[allow(dead_code)]
#[derive(DebugPretty, DisplaySimple, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LoginResponse {
    /// The access token.
    pub access_token: String,
    /// The token type.
    pub token_type: String,
    /// The expiration time in seconds.
    pub expires_in: u32,
}
