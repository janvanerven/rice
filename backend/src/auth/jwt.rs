use chrono::{TimeDelta, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub jti: String,  // session_id for revocation
    pub email: String,
    pub exp: i64,
    pub iat: i64,
}

pub fn create_access_token(
    user_id: &str,
    session_id: &str,
    email: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        jti: session_id.to_string(),
        email: email.to_string(),
        exp: (now + TimeDelta::minutes(15)).timestamp(),
        iat: now.timestamp(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn decode_ignoring_expiry(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.validate_exp = false;
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let secret = "a]very]secret]key]that]is]at]least]32]bytes";
        let token =
            create_access_token("user123", "session456", "test@example.com", secret).unwrap();
        let claims = verify_token(&token, secret).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.jti, "session456");
        assert_eq!(claims.email, "test@example.com");
    }

    #[test]
    fn test_verify_with_wrong_secret() {
        let token = create_access_token(
            "user123",
            "session456",
            "test@example.com",
            "a]very]secret]key]that]is]at]least]32]bytes",
        )
        .unwrap();
        let result = verify_token(&token, "wrong_secret_that_is_also_32_bytes_long!");
        assert!(result.is_err());
    }
}
