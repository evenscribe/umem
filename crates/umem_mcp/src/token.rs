use jsonwebtoken::{DecodingKey, TokenData, decode_header};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JWK {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub n: String,
    pub e: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JWKS {
    pub keys: Vec<JWK>,
}

pub async fn get_jwks(jwks_url: String) -> JWKS {
    let jwks_resp = reqwest::get(jwks_url).await.unwrap();
    jwks_resp.json().await.unwrap()
}

pub async fn check_token(token: &str, keys: &JWKS) -> Result<TokenData<Claims>, String> {
    let header = decode_header(token).unwrap();
    let kid = header.kid.ok_or("No kid found in token header")?;

    let client_id = match std::env::var("WORKOS_CLIENT_ID") {
        Ok(id) => id,
        Err(_) => return Err("WORKOS_CLIENT_ID environment variable not set".to_string()),
    };

    let jwk = keys
        .keys
        .iter()
        .find(|k| k.kid == kid)
        .ok_or("No matching kid found in jwks")?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|op| format!("Decoding Key Error: {:?}", op))?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[client_id]);

    let token_data = jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|op| format!("JWT Decode Error: {:?}", op))?;

    Ok(token_data)
}
