//! Public key validation.

/// Ensure that the given public key is a valid ed25519 key.
///
/// Return an error string if the key is invalid.
pub fn validate_public_key(public_key: &str) -> Result<(), String> {
    // Ensure the ID starts with the correct sigil link.
    if !public_key.starts_with('@') {
        return Err("expected '@' sigil as first character".to_string());
    }

    // Find the dot index denoting the start of the algorithm definition tag.
    let dot_index = match public_key.rfind('.') {
        Some(index) => index,
        None => return Err("no dot index was found".to_string()),
    };

    // Check the hashing algorithm (must end with ".ed25519").
    if !&public_key.ends_with(".ed25519") {
        return Err("hashing algorithm must be ed25519".to_string());
    }

    // Obtain the base64 portion (substring) of the public key.
    let base64_str = &public_key[1..dot_index];

    // Ensure the length of the base64 encoded ed25519 public key is correct.
    if base64_str.len() != 44 {
        return Err("base64 data length is incorrect".to_string());
    }

    Ok(())
}
