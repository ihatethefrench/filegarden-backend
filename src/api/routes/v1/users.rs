//! An HTTP resource representing the set of all user accounts.

use argon2::{
    password_hash::{Salt, SaltString},
    Argon2, PasswordHasher,
};
use axum_macros::debug_handler;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use lettre::Address;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use time::Date;
use validator::Validate;

use crate::{
    api::{validate::deserialize_date, Json, Response},
    db,
};

/// The length of a user ID in bytes.
const USER_ID_LENGTH: usize = 8;

/// A `POST` request body.
#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PostRequest {
    /// The user's email address.
    pub email: Address,

    /// The user's name.
    #[validate(length(min = 1, max = 64))]
    pub name: String,

    /// The user's birthdate, from a string in ISO 8601 date format.
    #[serde(deserialize_with = "deserialize_date")]
    pub birthdate: Date,

    /// The user's password in plain text.
    #[validate(length(min = 8, max = 256))]
    pub password: String,
}

/// A `POST` response body.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostResponse {
    /// The user's ID.
    pub id: String,
}

/// Creates a new user and sends them an account verification email.
///
/// # Errors
///
/// See [`api::Error`].
#[debug_handler]
pub async fn post(Json(body): Json<PostRequest>) -> Response<PostResponse> {
    let user_id = {
        let mut user_id = [0_u8; USER_ID_LENGTH];
        rand::thread_rng().try_fill_bytes(&mut user_id)?;
        user_id
    };

    let salt_string = SaltString::encode_b64(&{
        let mut salt = [0_u8; Salt::RECOMMENDED_LENGTH];
        rand::thread_rng().try_fill_bytes(&mut salt)?;
        salt
    })
    .expect("salt should be valid");

    let password_hash = Argon2::default()
        .hash_password(body.password.as_bytes(), &salt_string)
        .expect("password hashing should be infallible")
        .to_string();

    sqlx::query!(
        "INSERT INTO users (id, email, name, birthdate, password_hash) VALUES ($1, $2, $3, $4, $5)",
        &user_id,
        body.email.to_string(),
        body.name,
        body.birthdate,
        password_hash,
    )
    .execute(db::pool())
    .await?;

    Ok(Json(PostResponse {
        id: URL_SAFE_NO_PAD.encode(user_id),
    }))
}
