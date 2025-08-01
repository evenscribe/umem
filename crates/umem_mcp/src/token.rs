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

    let jwks: JWKS = jwks_resp.json().await.unwrap();

    jwks
}

pub async fn check_token(token: &str, keys: &JWKS) -> Result<TokenData<Claims>, String> {
    let header = decode_header(token).unwrap();

    let kid = header.kid.ok_or("No kid found in token header")?;

    let jwk = keys
        .keys
        .iter()
        .find(|k| k.kid == kid)
        .ok_or("No matching kid found in jwks")?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|op| format!("Error: {:?}", op))?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);

    validation.validate_exp;

    let token_data = jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation)
        .map_err(|op| format!("Error: {:?}", op))?;

    println!("{:?}", token_data.claims);

    Ok(token_data)
}
