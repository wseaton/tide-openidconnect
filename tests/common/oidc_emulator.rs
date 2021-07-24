use std::collections::HashMap;

use async_lock::Mutex;
use async_std::prelude::*;
use async_std::sync::Arc;
use chrono::{Duration, Utc};
use openidconnect::{core::CoreIdTokenClaims, IssuerUrl, RedirectUrl};
use portpicker::pick_unused_port;
use tide::prelude::*;
use tide::Request;

use crate::common::authorizeurl::ParsedAuthorizeUrl;

// From here: <https://github.com/ramosbugs/openidconnect-rs/blob/cfa5af581ee100791f68bf099dd15fa3eb492c8b/src/jwt.rs#L489>
const TEST_RSA_PUB_KEY: &str = "{
            \"kty\": \"RSA\",
            \"kid\": \"bilbo.baggins@hobbiton.example\",
            \"use\": \"sig\",
            \"n\": \"n4EPtAOCc9AlkeQHPzHStgAbgs7bTZLwUBZdR8_KuKPEHLd4rHVTeT\
                     -O-XV2jRojdNhxJWTDvNd7nqQ0VEiZQHz_AJmSCpMaJMRBSFKrKb2wqV\
                     wGU_NsYOYL-QtiWN2lbzcEe6XC0dApr5ydQLrHqkHHig3RBordaZ6Aj-\
                     oBHqFEHYpPe7Tpe-OfVfHd1E6cS6M1FZcD1NNLYD5lFHpPI9bTwJlsde\
                     3uhGqC0ZCuEHg8lhzwOHrtIQbS0FVbb9k3-tVTU4fg_3L_vniUFAKwuC\
                     LqKnS2BYwdq_mzSnbLY7h_qixoR7jig3__kRhuaxwUkRz5iaiQkqgc5g\
                     HdrNP5zw\",
            \"e\": \"AQAB\"
        }";

const TEST_RSA_PRIV_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
         MIIEowIBAAKCAQEAn4EPtAOCc9AlkeQHPzHStgAbgs7bTZLwUBZdR8/KuKPEHLd4\n\
         rHVTeT+O+XV2jRojdNhxJWTDvNd7nqQ0VEiZQHz/AJmSCpMaJMRBSFKrKb2wqVwG\n\
         U/NsYOYL+QtiWN2lbzcEe6XC0dApr5ydQLrHqkHHig3RBordaZ6Aj+oBHqFEHYpP\n\
         e7Tpe+OfVfHd1E6cS6M1FZcD1NNLYD5lFHpPI9bTwJlsde3uhGqC0ZCuEHg8lhzw\n\
         OHrtIQbS0FVbb9k3+tVTU4fg/3L/vniUFAKwuCLqKnS2BYwdq/mzSnbLY7h/qixo\n\
         R7jig3//kRhuaxwUkRz5iaiQkqgc5gHdrNP5zwIDAQABAoIBAG1lAvQfhBUSKPJK\n\
         Rn4dGbshj7zDSr2FjbQf4pIh/ZNtHk/jtavyO/HomZKV8V0NFExLNi7DUUvvLiW7\n\
         0PgNYq5MDEjJCtSd10xoHa4QpLvYEZXWO7DQPwCmRofkOutf+NqyDS0QnvFvp2d+\n\
         Lov6jn5C5yvUFgw6qWiLAPmzMFlkgxbtjFAWMJB0zBMy2BqjntOJ6KnqtYRMQUxw\n\
         TgXZDF4rhYVKtQVOpfg6hIlsaoPNrF7dofizJ099OOgDmCaEYqM++bUlEHxgrIVk\n\
         wZz+bg43dfJCocr9O5YX0iXaz3TOT5cpdtYbBX+C/5hwrqBWru4HbD3xz8cY1TnD\n\
         qQa0M8ECgYEA3Slxg/DwTXJcb6095RoXygQCAZ5RnAvZlno1yhHtnUex/fp7AZ/9\n\
         nRaO7HX/+SFfGQeutao2TDjDAWU4Vupk8rw9JR0AzZ0N2fvuIAmr/WCsmGpeNqQn\n\
         ev1T7IyEsnh8UMt+n5CafhkikzhEsrmndH6LxOrvRJlsPp6Zv8bUq0kCgYEAuKE2\n\
         dh+cTf6ERF4k4e/jy78GfPYUIaUyoSSJuBzp3Cubk3OCqs6grT8bR/cu0Dm1MZwW\n\
         mtdqDyI95HrUeq3MP15vMMON8lHTeZu2lmKvwqW7anV5UzhM1iZ7z4yMkuUwFWoB\n\
         vyY898EXvRD+hdqRxHlSqAZ192zB3pVFJ0s7pFcCgYAHw9W9eS8muPYv4ZhDu/fL\n\
         2vorDmD1JqFcHCxZTOnX1NWWAj5hXzmrU0hvWvFC0P4ixddHf5Nqd6+5E9G3k4E5\n\
         2IwZCnylu3bqCWNh8pT8T3Gf5FQsfPT5530T2BcsoPhUaeCnP499D+rb2mTnFYeg\n\
         mnTT1B/Ue8KGLFFfn16GKQKBgAiw5gxnbocpXPaO6/OKxFFZ+6c0OjxfN2PogWce\n\
         TU/k6ZzmShdaRKwDFXisxRJeNQ5Rx6qgS0jNFtbDhW8E8WFmQ5urCOqIOYk28EBi\n\
         At4JySm4v+5P7yYBh8B8YD2l9j57z/s8hJAxEbn/q8uHP2ddQqvQKgtsni+pHSk9\n\
         XGBfAoGBANz4qr10DdM8DHhPrAb2YItvPVz/VwkBd1Vqj8zCpyIEKe/07oKOvjWQ\n\
         SgkLDH9x2hBgY01SbP43CvPk0V72invu2TGkI/FXwXWJLLG7tDSgw4YyfhrYrHmg\n\
         1Vre3XB9HH8MYBVB6UIexaAq4xSeoemRKTBesZro7OKjKT8/GmiO\
         -----END RSA PRIVATE KEY-----";

struct Token {
    access_token: String,
    scopes: String,
    userid: String,
    nonce: String,
}

fn create_id_token(
    issuer_url: &IssuerUrl,
    userid: impl AsRef<str>,
    nonce: impl AsRef<str>,
) -> openidconnect::IdToken<
    openidconnect::EmptyAdditionalClaims,
    openidconnect::core::CoreGenderClaim,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJwsSigningAlgorithm,
    openidconnect::core::CoreJsonWebKeyType,
> {
    let claims = CoreIdTokenClaims::new(
        issuer_url.clone(),
        vec![openidconnect::Audience::new("CLIENT-ID".to_string())],
        Utc::now().checked_add_signed(Duration::hours(1)).unwrap(),
        Utc::now(),
        openidconnect::StandardClaims::new(openidconnect::SubjectIdentifier::new(
            userid.as_ref().to_string(),
        )),
        openidconnect::EmptyAdditionalClaims {},
    )
    .set_nonce(Some(openidconnect::Nonce::new(nonce.as_ref().to_string())));

    openidconnect::core::CoreIdToken::new(
        claims,
        &openidconnect::core::CoreRsaPrivateSigningKey::from_pem(TEST_RSA_PRIV_KEY, None).unwrap(),
        openidconnect::core::CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        None,
        None,
    )
    .unwrap()
}

pub struct OpenIdConnectEmulator {
    /// Redirect URL to which the client is sent at the end of the OpenID
    /// Connect process.
    redirect_url: RedirectUrl,

    /// TCP Port on which the OIDC emulator responds to HTTP requests.
    port: u16,

    /// Tokens available for request on this server, indexed by authorization
    /// code.
    tokens: Arc<Mutex<HashMap<String, Token>>>,
}

#[derive(Clone)]
struct State {
    /// Issuer URL associated with the tokens generated by this emulator.
    issuer_url: IssuerUrl,

    /// Tokens available for request on this server, indexed by authorization
    /// code.
    tokens: Arc<Mutex<HashMap<String, Token>>>,
}

impl OpenIdConnectEmulator {
    pub fn new(redirect_url: RedirectUrl) -> Self {
        Self {
            redirect_url,
            port: pick_unused_port().expect("No ports free"),
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn issuer_url(&self) -> IssuerUrl {
        IssuerUrl::new(format!("http://localhost:{}/", self.port)).unwrap()
    }

    pub async fn run_with_emulator<'a, Fut>(
        &'a self,
        f: impl FnOnce(&'a Self) -> Fut,
    ) -> http_types::Result<()>
    where
        Fut: Future<Output = http_types::Result<()>>,
    {
        self.run().race(f(self)).await
    }

    pub async fn run(&self) -> http_types::Result<()> {
        let state = State {
            issuer_url: self.issuer_url(),
            tokens: Arc::clone(&self.tokens),
        };
        let mut app = tide::with_state(state);

        let oidc_port = self.port;
        app.at("/.well-known/openid-configuration").get(
                move |_req: Request<State>| async move {
                    Ok(json!({
                            "issuer": format!("http://localhost:{}/", oidc_port),
                            "authorization_endpoint": format!("http://localhost:{}/authorization", oidc_port),
                            "token_endpoint": format!("http://localhost:{}/token", oidc_port),
                            "jwks_uri": format!("http://localhost:{}/jwks", oidc_port),
                            "response_types_supported": ["code"],
                            "subject_types_supported": ["public"],
                            "id_token_signing_alg_values_supported": ["RS256"]
                    }))
                },
            );

        app.at("/jwks").get(move |_req: Request<State>| async move {
            Ok(json!({
                        "keys": [{
                            "kty": "RSA",
                            "kid": "bilbo.baggins@hobbiton.example",
                            "use": "sig",
                            "n": "n4EPtAOCc9AlkeQHPzHStgAbgs7bTZLwUBZdR8_KuKPEHLd4rHVTeT\
                                  -O-XV2jRojdNhxJWTDvNd7nqQ0VEiZQHz_AJmSCpMaJMRBSFKrKb2wqV\
                                  wGU_NsYOYL-QtiWN2lbzcEe6XC0dApr5ydQLrHqkHHig3RBordaZ6Aj-\
                                  oBHqFEHYpPe7Tpe-OfVfHd1E6cS6M1FZcD1NNLYD5lFHpPI9bTwJlsde\
                                  3uhGqC0ZCuEHg8lhzwOHrtIQbS0FVbb9k3-tVTU4fg_3L_vniUFAKwuC\
                                  LqKnS2BYwdq_mzSnbLY7h_qixoR7jig3__kRhuaxwUkRz5iaiQkqgc5g\
                                  HdrNP5zw",
                            "e": "AQAB"}]}))
        });

        app.at("/token")
            .post(move |mut req: Request<State>| async move {
                // Get the authorization code from the request.
                #[derive(Deserialize)]
                struct TokenRequest {
                    code: String,
                }
                let token_request: TokenRequest = req.body_form().await?;

                // Find and return the token linked to this code (or an
                // error if we cannot find the code).
                let tokens = req.state().tokens.lock().await;
                if let Some(token) = tokens.get(&token_request.code) {
                    Ok(json!({
                        "access_token": token.access_token,
                        "token_type": "bearer",
                        "scope": token.scopes,
                        "id_token": create_id_token(&req.state().issuer_url, &token.userid, &token.nonce)
                    }))
                } else {
                    Err(tide::http::Error::from_str(
                        tide::StatusCode::InternalServerError,
                        "Invalid authorization code.",
                    ))
                }
            });

        app.listen(format!("tcp://localhost:{}", self.port)).await?;
        Ok(())
    }

    pub async fn add_token<S>(
        &self,
        access_token: S,
        scopes: S,
        userid: S,
        authorize_url: &ParsedAuthorizeUrl,
    ) -> String
    where
        S: AsRef<str>,
    {
        // TODO Generate a random (GUID?) authorization_code.
        let authorization_code = "12345";

        // Create the token and add it to the emulator.
        let mut tokens = self.tokens.lock().await;
        tokens.insert(
            authorization_code.to_string(),
            Token {
                access_token: access_token.as_ref().to_string(),
                scopes: scopes.as_ref().to_string(),
                userid: userid.as_ref().to_string(),
                nonce: authorize_url.nonce.as_ref().unwrap().to_string(),
            },
        );

        // Return the callback URL (back to the application-under-test)
        // which will complete the auth request by exchanging the code for
        // the token.
        format!(
            "{}?code={}&state={}",
            self.redirect_url.url().path(),
            authorization_code,
            authorize_url.state.as_ref().unwrap(),
        )
    }
}
