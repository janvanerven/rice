use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointNotSet, EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde::Deserialize;

use crate::config::Config;

/// Fully-configured OAuth client with auth URL and token URL set.
pub type ConfiguredClient = BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Debug, Deserialize)]
pub struct AuthentikUserInfo {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
    pub picture: Option<String>,
}

pub fn build_oauth_client(config: &Config) -> Result<ConfiguredClient, String> {
    let base = config.authentik_base_url.trim_end_matches('/');
    let app = config.app_base_url.trim_end_matches('/');

    let auth_url = AuthUrl::new(format!("{base}/application/o/authorize/"))
        .map_err(|e| format!("Invalid AUTHENTIK_BASE_URL: {e}"))?;
    let token_url = TokenUrl::new(format!("{base}/application/o/token/"))
        .map_err(|e| format!("Invalid AUTHENTIK_BASE_URL: {e}"))?;
    let redirect_url = RedirectUrl::new(format!("{app}/auth/callback"))
        .map_err(|e| format!("Invalid APP_BASE_URL: {e}"))?;

    Ok(BasicClient::new(ClientId::new(config.authentik_client_id.clone()))
        .set_client_secret(ClientSecret::new(config.authentik_client_secret.clone()))
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_redirect_uri(redirect_url))
}

pub fn generate_auth_url(client: &ConfiguredClient) -> (String, CsrfToken, PkceCodeVerifier) {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".into()))
        .add_scope(Scope::new("profile".into()))
        .add_scope(Scope::new("email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url.to_string(), csrf_token, pkce_verifier)
}

pub async fn exchange_code(
    client: &ConfiguredClient,
    code: String,
    pkce_verifier: PkceCodeVerifier,
) -> Result<String, String> {
    let http_client = oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
        .map_err(|e| format!("Token exchange failed: {e}"))?;

    Ok(token_result.access_token().secret().clone())
}

pub async fn fetch_user_info(
    authentik_base_url: &str,
    access_token: &str,
) -> Result<AuthentikUserInfo, String> {
    let url = format!(
        "{}/application/o/userinfo/",
        authentik_base_url.trim_end_matches('/')
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| format!("Userinfo request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Userinfo returned status: {}", resp.status()));
    }

    resp.json::<AuthentikUserInfo>()
        .await
        .map_err(|e| format!("Failed to parse userinfo: {e}"))
}
