#![no_std]
use soroban_sdk::{Env, String, Symbol, Map, Vec};
use crate::StringValidationError;

/// String validation utilities for scholarship metadata
/// Provides robust validation against empty, malformed, or malicious strings

pub const MIN_STRING_LENGTH: u32 = 1;
pub const MAX_STRING_LENGTH: u32 = 256;
pub const MAX_METADATA_VALUE_LENGTH: u32 = 512;
pub const MAX_STUDENT_ID_LENGTH: u32 = 128;
pub const MAX_ACHIEVEMENT_TITLE_LENGTH: u32 = 100;
pub const MAX_ACHIEVEMENT_DESC_LENGTH: u32 = 500;
pub const MAX_ICON_URL_LENGTH: u32 = 256;
pub const MAX_CATEGORY_LENGTH: u32 = 50;

/// Allowed characters for student IDs (alphanumeric + @._+-)
const STUDENT_ID_ALLOWED_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789@._+-";

/// Common malicious patterns to block
const MALICIOUS_PATTERNS: [&str; 8] = [
    "<script",
    "javascript:",
    "data:",
    "vbscript:",
    "onload=",
    "onerror=",
    "onclick=",
    "eval(",
];

/// Validates a basic string field
pub fn validate_string(env: &Env, input: &String, field_name: &str) -> Result<(), StringValidationError> {
    let len = input.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len < MIN_STRING_LENGTH as usize {
        return Err(StringValidationError::TooShort(
            field_name.to_string(),
            MIN_STRING_LENGTH,
            len as u32,
        ));
    }
    
    if len > MAX_STRING_LENGTH as usize {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            MAX_STRING_LENGTH,
            len as u32,
        ));
    }
    
    // Check for malicious patterns
    let input_str = input.to_string();
    for pattern in MALICIOUS_PATTERNS.iter() {
        if input_str.contains(pattern) {
            return Err(StringValidationError::MaliciousContent(
                field_name.to_string(),
                pattern.to_string(),
            ));
        }
    }
    
    Ok(())
}

/// Validates student ID with specific constraints
pub fn validate_student_id(env: &Env, student_id: &String) -> Result<(), StringValidationError> {
    let field_name = "student_id";
    let len = student_id.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len > MAX_STUDENT_ID_LENGTH as usize {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            MAX_STUDENT_ID_LENGTH,
            len as u32,
        ));
    }
    
    // Check for allowed characters only
    let student_id_str = student_id.to_string();
    for (i, c) in student_id_str.chars().enumerate() {
        if !STUDENT_ID_ALLOWED_CHARS.contains(c) {
            return Err(StringValidationError::InvalidCharacter(
                field_name.to_string(),
                c.to_string(),
                i as u32,
            ));
        }
    }
    
    // Basic email format validation if contains @
    if student_id_str.contains('@') {
        let parts: Vec<&str> = student_id_str.split('@').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(StringValidationError::InvalidFormat(
                field_name.to_string(),
                "Invalid email format".to_string(),
            ));
        }
        
        // Check domain part has at least one dot
        if !parts[1].contains('.') {
            return Err(StringValidationError::InvalidFormat(
                field_name.to_string(),
                "Email domain must contain a dot".to_string(),
            ));
        }
    }
    
    Ok(())
}

/// Validates achievement title
pub fn validate_achievement_title(env: &Env, title: &String) -> Result<(), StringValidationError> {
    let field_name = "achievement_title";
    let len = title.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len > MAX_ACHIEVEMENT_TITLE_LENGTH as usize {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            MAX_ACHIEVEMENT_TITLE_LENGTH,
            len as u32,
        ));
    }
    
    // Check for malicious patterns
    let title_str = title.to_string();
    for pattern in MALICIOUS_PATTERNS.iter() {
        if title_str.contains(pattern) {
            return Err(StringValidationError::MaliciousContent(
                field_name.to_string(),
                pattern.to_string(),
            ));
        }
    }
    
    Ok(())
}

/// Validates achievement description
pub fn validate_achievement_description(env: &Env, description: &String) -> Result<(), StringValidationError> {
    let field_name = "achievement_description";
    let len = description.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len > MAX_ACHIEVEMENT_DESC_LENGTH as usize {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            MAX_ACHIEVEMENT_DESC_LENGTH,
            len as u32,
        ));
    }
    
    // Check for malicious patterns
    let desc_str = description.to_string();
    for pattern in MALICIOUS_PATTERNS.iter() {
        if desc_str.contains(pattern) {
            return Err(StringValidationError::MaliciousContent(
                field_name.to_string(),
                pattern.to_string(),
            ));
        }
    }
    
    Ok(())
}

/// Validates achievement icon URL
pub fn validate_achievement_icon(env: &Env, icon: &String) -> Result<(), StringValidationError> {
    let field_name = "achievement_icon";
    let len = icon.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len > MAX_ICON_URL_LENGTH as usize {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            MAX_ICON_URL_LENGTH,
            len as u32,
        ));
    }
    
    // Basic URL validation
    let icon_str = icon.to_string();
    if !(icon_str.starts_with("http://") || icon_str.starts_with("https://") || icon_str.starts_with("ipfs://")) {
        return Err(StringValidationError::InvalidFormat(
            field_name.to_string(),
            "Icon must be a valid URL (http, https, or ipfs)".to_string(),
        ));
    }
    
    Ok(())
}

/// Validates achievement category
pub fn validate_achievement_category(env: &Env, category: &String) -> Result<(), StringValidationError> {
    let field_name = "achievement_category";
    let len = category.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len > MAX_CATEGORY_LENGTH as usize {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            MAX_CATEGORY_LENGTH,
            len as u32,
        ));
    }
    
    // Only allow alphanumeric and spaces
    let category_str = category.to_string();
    for (i, c) in category_str.chars().enumerate() {
        if !c.is_alphanumeric() && c != ' ' && c != '-' && c != '_' {
            return Err(StringValidationError::InvalidCharacter(
                field_name.to_string(),
                c.to_string(),
                i as u32,
            ));
        }
    }
    
    Ok(())
}

/// Validates metadata map keys and values
pub fn validate_metadata(env: &Env, metadata: &Map<Symbol, String>) -> Result<(), StringValidationError> {
    if metadata.is_empty() {
        return Err(StringValidationError::EmptyMetadata);
    }
    
    // Check metadata size (prevent storage bloat)
    if metadata.len() > 50 {
        return Err(StringValidationError::MetadataTooLarge(metadata.len()));
    }
    
    for (key, value) in metadata.iter() {
        let key_str = key.to_string();
        
        // Validate key
        if key_str.is_empty() {
            return Err(StringValidationError::EmptyMetadataKey);
        }
        
        if key_str.len() > 100 {
            return Err(StringValidationError::MetadataKeyTooLong(key_str.len()));
        }
        
        // Validate value
        validate_string(env, &value, &format!("metadata[{}]", key_str))?;
        
        if value.len() > MAX_METADATA_VALUE_LENGTH as usize {
            return Err(StringValidationError::MetadataValueTooLong(
                key_str,
                MAX_METADATA_VALUE_LENGTH,
                value.len() as u32,
            ));
        }
    }
    
    Ok(())
}

/// Validates achievement rarity
pub fn validate_achievement_rarity(env: &Env, rarity: &String) -> Result<(), StringValidationError> {
    let field_name = "achievement_rarity";
    let len = rarity.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    let rarity_str = rarity.to_string();
    let valid_rarities = ["common", "uncommon", "rare", "epic", "legendary"];
    
    if !valid_rarities.contains(&rarity_str.as_str()) {
        return Err(StringValidationError::InvalidRarity(rarity_str));
    }
    
    Ok(())
}

/// Comprehensive validation for Achievement struct
pub fn validate_achievement_complete(
    env: &Env,
    achievement_id: &String,
    title: &String,
    description: &String,
    icon: &String,
    category: &String,
    rarity: &String,
) -> Result<(), StringValidationError> {
    // Validate achievement ID
    validate_string(env, achievement_id, "achievement_id")?;
    
    // Validate other fields
    validate_achievement_title(env, title)?;
    validate_achievement_description(env, description)?;
    validate_achievement_icon(env, icon)?;
    validate_achievement_category(env, category)?;
    validate_achievement_rarity(env, rarity)?;
    
    Ok(())
}

/// Helper function to panic with appropriate validation error
pub fn panic_with_validation_error(env: &Env, error: StringValidationError) {
    env.panic_with_error(error);
}

/// Validate string with automatic panic on error (for convenience in contract functions)
pub fn validate_string_or_panic(env: &Env, input: &String, field_name: &str) {
    if let Err(error) = validate_string(env, input, field_name) {
        panic_with_validation_error(env, error);
    }
}

/// Validate student ID with automatic panic on error
pub fn validate_student_id_or_panic(env: &Env, student_id: &String) {
    if let Err(error) = validate_student_id(env, student_id) {
        panic_with_validation_error(env, error);
    }
}

/// Validate metadata with automatic panic on error
pub fn validate_metadata_or_panic(env: &Env, metadata: &Map<Symbol, String>) {
    if let Err(error) = validate_metadata(env, metadata) {
        panic_with_validation_error(env, error);
    }
}

/// Validate achievement complete with automatic panic on error
pub fn validate_achievement_complete_or_panic(
    env: &Env,
    achievement_id: &String,
    title: &String,
    description: &String,
    icon: &String,
    category: &String,
    rarity: &String,
) {
    if let Err(error) = validate_achievement_complete(env, achievement_id, title, description, icon, category, rarity) {
        panic_with_validation_error(env, error);
    }
}

/// Validates a Symbol field (used for reasons, categories, etc.)
pub fn validate_symbol(env: &Env, symbol: &Symbol, field_name: &str) -> Result<(), StringValidationError> {
    let symbol_str = symbol.to_string();
    let len = symbol_str.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(field_name.to_string()));
    }
    
    if len > 100 {
        return Err(StringValidationError::TooLong(
            field_name.to_string(),
            100,
            len as u32,
        ));
    }
    
    // Check for malicious patterns
    for pattern in MALICIOUS_PATTERNS.iter() {
        if symbol_str.contains(pattern) {
            return Err(StringValidationError::MaliciousContent(
                field_name.to_string(),
                pattern.to_string(),
            ));
        }
    }
    
    // Only allow alphanumeric, spaces, and basic punctuation
    for (i, c) in symbol_str.chars().enumerate() {
        if !c.is_alphanumeric() && c != ' ' && c != '-' && c != '_' && c != '.' {
            return Err(StringValidationError::InvalidCharacter(
                field_name.to_string(),
                c.to_string(),
                i as u32,
            ));
        }
    }
    
    Ok(())
}

/// Validate Symbol with automatic panic on error
pub fn validate_symbol_or_panic(env: &Env, symbol: &Symbol, field_name: &str) {
    if let Err(error) = validate_symbol(env, symbol, field_name) {
        panic_with_validation_error(env, error);
    }
}

/// Validates Bytes field (used for evidence, reasons, hashes)
pub fn validate_bytes(env: &Env, bytes: &soroban_sdk::Bytes, field_name: &str) -> Result<(), StringValidationError> {
    let len = bytes.len();
    
    if len == 0 {
        return Err(StringValidationError::EmptyString(format!("{}_bytes", field_name)));
    }
    
    // Maximum size for Bytes fields (prevent storage bloat)
    if len > 1024 {
        return Err(StringValidationError::TooLong(
            format!("{}_bytes", field_name),
            1024,
            len as u32,
        ));
    }
    
    Ok(())
}

/// Validate Bytes with automatic panic on error
pub fn validate_bytes_or_panic(env: &Env, bytes: &soroban_sdk::Bytes, field_name: &str) {
    if let Err(error) = validate_bytes(env, bytes, field_name) {
        panic_with_validation_error(env, error);
    }
}

/// Validates BytesN field (fixed-size bytes)
pub fn validate_bytes_n<const N: usize>(env: &Env, bytes: &soroban_sdk::BytesN<N>, field_name: &str) -> Result<(), StringValidationError> {
    // BytesN is fixed size, so we just check it's not all zeros (empty)
    let bytes_array = bytes.to_array();
    let all_zeros = bytes_array.iter().all(|&b| b == 0);
    
    if all_zeros {
        return Err(StringValidationError::EmptyString(format!("{}_bytes", field_name)));
    }
    
    Ok(())
}

/// Validate BytesN with automatic panic on error
pub fn validate_bytes_n_or_panic<const N: usize>(env: &Env, bytes: &soroban_sdk::BytesN<N>, field_name: &str) {
    if let Err(error) = validate_bytes_n(env, bytes, field_name) {
        panic_with_validation_error(env, error);
    }
}

#[cfg(test)]
mod tests;
