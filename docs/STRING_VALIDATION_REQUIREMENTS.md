# String Validation Requirements for Stream-Scholar Contracts

This document outlines the comprehensive string validation requirements implemented to ensure security, reliability, and data integrity in scholarship metadata.

## Overview

The Stream-Scholar smart contracts now include robust validation against empty or malformed strings in all scholarship metadata fields. This prevents security vulnerabilities, ensures data consistency, and provides clear error messages for developers.

## Validation Rules

### Student ID Validation

**Purpose**: Validates student identifiers used for profile creation and lookup.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Length**: 128 characters
- **Allowed Characters**: Alphanumeric characters plus `@._+-`
- **Email Format**: If contains `@`, must follow email format with domain containing a dot
- **Error Codes**: 601-606

**Examples**:
```rust
// Valid
"student123"
"student@university.edu"
"student.name@school.edu"

// Invalid
""
"student#123"  // Invalid character
"student@"     // Invalid email format
"student@domain" // Domain missing dot
```

### Achievement Title Validation

**Purpose**: Validates achievement titles in student profiles.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Length**: 100 characters
- **Security**: Blocks malicious patterns (script tags, JavaScript, etc.)
- **Error Codes**: 601, 603, 605

**Examples**:
```rust
// Valid
"First Course Completion"
"Honor Roll Student"

// Invalid
""
"<script>alert('xss')</script>"
"A".repeat(150)  // Too long
```

### Achievement Description Validation

**Purpose**: Validates achievement descriptions.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Length**: 500 characters
- **Security**: Blocks malicious patterns
- **Error Codes**: 601, 603, 605

### Achievement Icon Validation

**Purpose**: Validates achievement icon URLs.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Length**: 256 characters
- **Allowed Protocols**: `http://`, `https://`, `ipfs://`
- **Error Codes**: 601, 603, 606

**Examples**:
```rust
// Valid
"https://example.com/icon.png"
"ipfs://QmHash123"

// Invalid
"ftp://example.com/icon.png"
"javascript:alert('xss')"
```

### Achievement Category Validation

**Purpose**: Validates achievement categories.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Length**: 50 characters
- **Allowed Characters**: Alphanumeric, spaces, hyphens, underscores
- **Error Codes**: 601, 603, 604

**Examples**:
```rust
// Valid
"academic"
"academic excellence"
"research-milestone"

// Invalid
"academic@excellence"
"category with special chars!"
```

### Achievement Rarity Validation

**Purpose**: Validates achievement rarity tiers.

**Rules**:
- **Required**: Cannot be empty
- **Allowed Values**: `common`, `uncommon`, `rare`, `epic`, `legendary`
- **Error Codes**: 601, 612

### Metadata Validation

**Purpose**: Validates key-value metadata pairs.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Entries**: 50 key-value pairs
- **Key Requirements**: Non-empty, maximum 100 characters
- **Value Requirements**: Maximum 512 characters each
- **Security**: Values must pass string validation
- **Error Codes**: 607-611

### Symbol Validation

**Purpose**: Validates symbol fields used for reasons, categories, etc.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Length**: 100 characters
- **Allowed Characters**: Alphanumeric, spaces, hyphens, underscores, periods
- **Security**: Blocks malicious patterns
- **Error Codes**: 601, 603, 604, 605

**Examples**:
```rust
// Valid
"investigation"
"security audit"
"routine-check"

// Invalid
""
"investigation<script>"
"reason@special"
```

### Bytes Validation

**Purpose**: Validates byte arrays for evidence, hashes, reasons.

**Rules**:
- **Required**: Cannot be empty
- **Maximum Size**: 1024 bytes
- **Error Codes**: 601, 603

## Security Features

### Malicious Content Detection

The validation system automatically blocks common attack vectors:

- **XSS Attempts**: `<script>`, `javascript:`, `onload=`, `onclick=`, `eval(`
- **Protocol Injection**: `data:`, `vbscript:`
- **Content Security**: Prevents HTML/JS injection in metadata

### Input Sanitization

- **Character Filtering**: Only allows appropriate character sets per field type
- **Length Limits**: Prevents storage bloat and DoS attacks
- **Format Validation**: Ensures structured data follows expected patterns

## Error Handling

### Error Codes

| Code | Error Type | Description |
|------|------------|-------------|
| 601 | EmptyString | Field cannot be empty |
| 602 | TooShort | Field below minimum length |
| 603 | TooLong | Field exceeds maximum length |
| 604 | InvalidCharacter | Contains disallowed character |
| 605 | MaliciousContent | Contains potentially malicious content |
| 606 | InvalidFormat | Incorrect format for field type |
| 607 | EmptyMetadata | Metadata map cannot be empty |
| 608 | MetadataTooLarge | Too many metadata entries |
| 609 | EmptyMetadataKey | Metadata key cannot be empty |
| 610 | MetadataKeyTooLong | Metadata key exceeds limit |
| 611 | MetadataValueTooLong | Metadata value exceeds limit |
| 612 | InvalidRarity | Invalid achievement rarity |

### Error Messages

The validation system provides descriptive error messages that include:
- Field name
- Specific validation rule violated
- Expected vs actual values (when applicable)
- Clear guidance for fixing the issue

## Implementation Details

### Validation Functions

The system provides both Result-based and panic-based functions:

```rust
// Result-based - for custom error handling
validate_string(&env, &input, "field_name")?;

// Panic-based - for automatic contract failure
validate_string_or_panic(&env, &input, "field_name");
```

### Integration Points

Validation is integrated into:

1. **StudentProfileNFT Contract**:
   - `mint_nft()` - student_id and metadata validation
   - `add_achievement()` - complete achievement validation
   - `update_xp()` - student_id validation
   - All read-only functions - student_id validation

2. **Main Scholarship Contract**:
   - `trigger_security_hold()` - reason symbol validation
   - `validate_disciplinary_payload()` - evidence and reason bytes validation

3. **Future Functions**:
   - Any new functions handling string data should include validation

## Best Practices

### For Developers

1. **Always Validate**: Use validation functions for all string inputs
2. **Use Panic Functions**: For contract functions where validation failure should revert the transaction
3. **Handle Errors Gracefully**: Provide clear error messages to users
4. **Test Edge Cases**: Include tests for empty, malformed, and malicious inputs

### For Security

1. **Input Validation**: Never trust external input
2. **Output Encoding**: Ensure proper encoding when displaying user data
3. **Regular Updates**: Keep validation patterns updated with new threats
4. **Audit Trails**: Log validation failures for security monitoring

## Migration Guide

### For Existing Code

1. **Add Imports**: Include `use crate::string_validation::*;`
2. **Replace Basic Checks**: Replace simple empty checks with comprehensive validation
3. **Update Error Handling**: Use new error codes for string validation failures
4. **Add Tests**: Include validation tests for all string-handling functions

### Example Migration

**Before**:
```rust
if student_id.is_empty() {
    panic!("Student ID cannot be empty");
}
```

**After**:
```rust
validate_student_id_or_panic(&env, &student_id);
```

## Testing

### Test Coverage

The validation system includes comprehensive tests covering:

- ✅ Valid inputs
- ✅ Empty inputs
- ✅ Length limits
- ✅ Invalid characters
- ✅ Malicious content
- ✅ Format validation
- ✅ Edge cases
- ✅ Error codes and messages

### Running Tests

```bash
cd contracts/scholar_contracts
cargo test string_validation
```

## Future Enhancements

Planned improvements to the validation system:

1. **Unicode Support**: Enhanced international character handling
2. **Regex Patterns**: More sophisticated pattern matching
3. **Custom Validators**: Allow project-specific validation rules
4. **Batch Validation**: Validate multiple fields efficiently
5. **Validation Caching**: Cache validation results for repeated inputs

## Support

For questions about string validation requirements:

1. Check this documentation first
2. Review the test files for examples
3. Examine the validation module source code
4. Create an issue for specific questions or enhancement requests

---

**Last Updated**: April 29, 2026  
**Version**: 1.0.0  
**Status**: Implemented and Tested
