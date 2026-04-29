#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, String, Symbol, Map, Vec, Bytes};

    fn setup_env() -> Env {
        Env::default()
    }

    #[test]
    fn test_validate_string_success() {
        let env = setup_env();
        let valid_string = String::from_str(&env, "valid_string");
        
        assert!(validate_string(&env, &valid_string, "test_field").is_ok());
    }

    #[test]
    fn test_validate_string_empty() {
        let env = setup_env();
        let empty_string = String::from_str(&env, "");
        
        let result = validate_string(&env, &empty_string, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::EmptyString(_)));
    }

    #[test]
    fn test_validate_string_too_long() {
        let env = setup_env();
        let long_string = String::from_str(&env, &"a".repeat(300));
        
        let result = validate_string(&env, &long_string, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::TooLong(_, _, _)));
    }

    #[test]
    fn test_validate_string_malicious_content() {
        let env = setup_env();
        let malicious_string = String::from_str(&env, "test<script>alert('xss')</script>");
        
        let result = validate_string(&env, &malicious_string, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::MaliciousContent(_, _)));
    }

    #[test]
    fn test_validate_student_id_valid_email() {
        let env = setup_env();
        let email = String::from_str(&env, "student@university.edu");
        
        assert!(validate_student_id(&env, &email).is_ok());
    }

    #[test]
    fn test_validate_student_id_valid_alphanumeric() {
        let env = setup_env();
        let student_id = String::from_str(&env, "student123");
        
        assert!(validate_student_id(&env, &student_id).is_ok());
    }

    #[test]
    fn test_validate_student_id_invalid_email_format() {
        let env = setup_env();
        let invalid_email = String::from_str(&env, "invalid@");
        
        let result = validate_student_id(&env, &invalid_email);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::InvalidFormat(_, _)));
    }

    #[test]
    fn test_validate_student_id_invalid_character() {
        let env = setup_env();
        let invalid_id = String::from_str(&env, "student#123");
        
        let result = validate_student_id(&env, &invalid_id);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::InvalidCharacter(_, _, _)));
    }

    #[test]
    fn test_validate_achievement_title_valid() {
        let env = setup_env();
        let title = String::from_str(&env, "First Course Completion");
        
        assert!(validate_achievement_title(&env, &title).is_ok());
    }

    #[test]
    fn test_validate_achievement_title_too_long() {
        let env = setup_env();
        let long_title = String::from_str(&env, &"A".repeat(150));
        
        let result = validate_achievement_title(&env, &long_title);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::TooLong(_, _, _)));
    }

    #[test]
    fn test_validate_achievement_description_valid() {
        let env = setup_env();
        let description = String::from_str(&env, "Successfully completed the first course with high marks");
        
        assert!(validate_achievement_description(&env, &description).is_ok());
    }

    #[test]
    fn test_validate_achievement_icon_valid_http() {
        let env = setup_env();
        let icon = String::from_str(&env, "https://example.com/icon.png");
        
        assert!(validate_achievement_icon(&env, &icon).is_ok());
    }

    #[test]
    fn test_validate_achievement_icon_valid_ipfs() {
        let env = setup_env();
        let icon = String::from_str(&env, "ipfs://QmHash123");
        
        assert!(validate_achievement_icon(&env, &icon).is_ok());
    }

    #[test]
    fn test_validate_achievement_icon_invalid_protocol() {
        let env = setup_env();
        let icon = String::from_str(&env, "ftp://example.com/icon.png");
        
        let result = validate_achievement_icon(&env, &icon);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::InvalidFormat(_, _)));
    }

    #[test]
    fn test_validate_achievement_category_valid() {
        let env = setup_env();
        let category = String::from_str(&env, "academic");
        
        assert!(validate_achievement_category(&env, &category).is_ok());
    }

    #[test]
    fn test_validate_achievement_category_with_space() {
        let env = setup_env();
        let category = String::from_str(&env, "academic excellence");
        
        assert!(validate_achievement_category(&env, &category).is_ok());
    }

    #[test]
    fn test_validate_achievement_category_invalid_character() {
        let env = setup_env();
        let category = String::from_str(&env, "academic@excellence");
        
        let result = validate_achievement_category(&env, &category);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::InvalidCharacter(_, _, _)));
    }

    #[test]
    fn test_validate_achievement_rarity_valid() {
        let env = setup_env();
        let valid_rarities = ["common", "uncommon", "rare", "epic", "legendary"];
        
        for rarity in valid_rarities.iter() {
            let rarity_str = String::from_str(&env, rarity);
            assert!(validate_achievement_rarity(&env, &rarity_str).is_ok());
        }
    }

    #[test]
    fn test_validate_achievement_rarity_invalid() {
        let env = setup_env();
        let invalid_rarity = String::from_str(&env, "mythic");
        
        let result = validate_achievement_rarity(&env, &invalid_rarity);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::InvalidRarity(_)));
    }

    #[test]
    fn test_validate_metadata_valid() {
        let env = setup_env();
        let mut metadata = Map::new(&env);
        
        metadata.set(Symbol::from_str(&env, "name"), String::from_str(&env, "John Doe"));
        metadata.set(Symbol::from_str(&env, "institution"), String::from_str(&env, "University"));
        
        assert!(validate_metadata(&env, &metadata).is_ok());
    }

    #[test]
    fn test_validate_metadata_empty() {
        let env = setup_env();
        let empty_metadata = Map::new(&env);
        
        let result = validate_metadata(&env, &empty_metadata);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::EmptyMetadata));
    }

    #[test]
    fn test_validate_metadata_too_large() {
        let env = setup_env();
        let mut metadata = Map::new(&env);
        
        // Add 60 entries (exceeds limit of 50)
        for i in 0..60 {
            let key = Symbol::from_str(&env, &format!("key{}", i));
            let value = String::from_str(&env, &format!("value{}", i));
            metadata.set(key, value);
        }
        
        let result = validate_metadata(&env, &metadata);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::MetadataTooLarge(_)));
    }

    #[test]
    fn test_validate_metadata_value_too_long() {
        let env = setup_env();
        let mut metadata = Map::new(&env);
        
        let long_value = String::from_str(&env, &"A".repeat(600)); // Exceeds 512 limit
        metadata.set(Symbol::from_str(&env, "test"), long_value);
        
        let result = validate_metadata(&env, &metadata);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::MetadataValueTooLong(_, _, _)));
    }

    #[test]
    fn test_validate_symbol_valid() {
        let env = setup_env();
        let symbol = Symbol::from_str(&env, "valid_reason");
        
        assert!(validate_symbol(&env, &symbol, "test_field").is_ok());
    }

    #[test]
    fn test_validate_symbol_with_spaces() {
        let env = setup_env();
        let symbol = Symbol::from_str(&env, "valid reason");
        
        assert!(validate_symbol(&env, &symbol, "test_field").is_ok());
    }

    #[test]
    fn test_validate_symbol_empty() {
        let env = setup_env();
        let empty_symbol = Symbol::from_str(&env, "");
        
        let result = validate_symbol(&env, &empty_symbol, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::EmptyString(_)));
    }

    #[test]
    fn test_validate_symbol_too_long() {
        let env = setup_env();
        let long_symbol = Symbol::from_str(&env, &"A".repeat(150));
        
        let result = validate_symbol(&env, &long_symbol, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::TooLong(_, _, _)));
    }

    #[test]
    fn test_validate_symbol_invalid_character() {
        let env = setup_env();
        let invalid_symbol = Symbol::from_str(&env, "invalid@symbol");
        
        let result = validate_symbol(&env, &invalid_symbol, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::InvalidCharacter(_, _, _)));
    }

    #[test]
    fn test_validate_bytes_valid() {
        let env = setup_env();
        let bytes = Bytes::from_slice(&env, b"valid_bytes");
        
        assert!(validate_bytes(&env, &bytes, "test_field").is_ok());
    }

    #[test]
    fn test_validate_bytes_empty() {
        let env = setup_env();
        let empty_bytes = Bytes::from_slice(&env, b"");
        
        let result = validate_bytes(&env, &empty_bytes, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::EmptyString(_)));
    }

    #[test]
    fn test_validate_bytes_too_large() {
        let env = setup_env();
        let large_bytes = Bytes::from_slice(&env, &vec![0u8; 2000]); // Exceeds 1024 limit
        
        let result = validate_bytes(&env, &large_bytes, "test_field");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StringValidationError::TooLong(_, _, _)));
    }

    #[test]
    fn test_validate_achievement_complete_valid() {
        let env = setup_env();
        let achievement_id = String::from_str(&env, "achievement_1");
        let title = String::from_str(&env, "First Course");
        let description = String::from_str(&env, "Completed first course successfully");
        let icon = String::from_str(&env, "https://example.com/icon.png");
        let category = String::from_str(&env, "academic");
        let rarity = String::from_str(&env, "common");
        
        assert!(validate_achievement_complete(
            &env,
            &achievement_id,
            &title,
            &description,
            &icon,
            &category,
            &rarity
        ).is_ok());
    }

    #[test]
    fn test_validate_achievement_complete_invalid_title() {
        let env = setup_env();
        let achievement_id = String::from_str(&env, "achievement_1");
        let invalid_title = String::from_str(&env, ""); // Empty title
        let description = String::from_str(&env, "Completed first course successfully");
        let icon = String::from_str(&env, "https://example.com/icon.png");
        let category = String::from_str(&env, "academic");
        let rarity = String::from_str(&env, "common");
        
        let result = validate_achievement_complete(
            &env,
            &achievement_id,
            &invalid_title,
            &description,
            &icon,
            &category,
            &rarity
        );
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "EmptyString")]
    fn test_validate_string_or_panic_empty() {
        let env = setup_env();
        let empty_string = String::from_str(&env, "");
        
        validate_string_or_panic(&env, &empty_string, "test_field");
    }

    #[test]
    #[should_panic(expected = "EmptyString")]
    fn test_validate_student_id_or_panic_empty() {
        let env = setup_env();
        let empty_student_id = String::from_str(&env, "");
        
        validate_student_id_or_panic(&env, &empty_student_id);
    }

    #[test]
    #[should_panic(expected = "EmptyMetadata")]
    fn test_validate_metadata_or_panic_empty() {
        let env = setup_env();
        let empty_metadata = Map::new(&env);
        
        validate_metadata_or_panic(&env, &empty_metadata);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(StringValidationError::EmptyString("test".to_string()).to_error_code(), 601);
        assert_eq!(StringValidationError::TooShort("test".to_string(), 1, 0).to_error_code(), 602);
        assert_eq!(StringValidationError::TooLong("test".to_string(), 10, 15).to_error_code(), 603);
        assert_eq!(StringValidationError::InvalidCharacter("test".to_string(), "x".to_string(), 1).to_error_code(), 604);
        assert_eq!(StringValidationError::MaliciousContent("test".to_string(), "script".to_string()).to_error_code(), 605);
        assert_eq!(StringValidationError::InvalidFormat("test".to_string(), "invalid".to_string()).to_error_code(), 606);
        assert_eq!(StringValidationError::EmptyMetadata.to_error_code(), 607);
        assert_eq!(StringValidationError::MetadataTooLarge(60).to_error_code(), 608);
        assert_eq!(StringValidationError::EmptyMetadataKey.to_error_code(), 609);
        assert_eq!(StringValidationError::MetadataKeyTooLong(150).to_error_code(), 610);
        assert_eq!(StringValidationError::MetadataValueTooLong("key".to_string(), 100, 150).to_error_code(), 611);
        assert_eq!(StringValidationError::InvalidRarity("mythic".to_string()).to_error_code(), 612);
    }

    #[test]
    fn test_error_messages() {
        let error = StringValidationError::EmptyString("field_name".to_string());
        let message = error.to_error_message();
        assert!(message.contains("field_name"));
        assert!(message.contains("cannot be empty"));
        
        let error = StringValidationError::TooLong("field_name".to_string(), 10, 15);
        let message = error.to_error_message();
        assert!(message.contains("field_name"));
        assert!(message.contains("too long"));
        assert!(message.contains("10"));
        assert!(message.contains("15"));
    }
}
