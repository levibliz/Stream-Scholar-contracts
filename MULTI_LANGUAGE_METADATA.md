# Multi-Language Course Metadata Support (Issue #46)

## Overview

This implementation adds support for multi-language course metadata in the Stream-Scholar contracts, allowing courses to store IPFS links for different language versions of the same course.

## Features

### Core Functionality
- **Multi-Language Support**: Courses can now store metadata in multiple languages
- **IPFS Integration**: Metadata stored via IPFS links for decentralized content storage
- **Admin Control**: Secure admin-only access for metadata management
- **Validation**: Built-in validation for language codes and IPFS links

### Supported Languages
The implementation supports 40+ ISO 639-1 language codes including:
- English (en), Spanish (es), French (fr), German (de), Italian (it)
- Portuguese (pt), Russian (ru), Japanese (ja), Chinese (zh), Korean (ko)
- Arabic (ar), Hindi (hi), Turkish (tr), Polish (pl), Dutch (nl)
- And many more...

## Data Structures

### CourseMetadata
```rust
pub struct CourseMetadata {
    pub language_code: Symbol,  // ISO 639-1 language code
    pub ipfs_link: Symbol,      // IPFS hash/link for this language version
    pub title: Symbol,          // Course title in this language
    pub description: Symbol,    // Course description in this language
    pub updated_at: u64,        // Last update timestamp
}
```

### Updated CourseInfo
```rust
pub struct CourseInfo {
    pub course_id: u64,
    pub created_at: u64,
    pub is_active: bool,
    pub creator: Address,
    pub default_language: Symbol,      // Default language code
    pub available_languages: Vec<Symbol>, // List of available language codes
}
```

## API Functions

### Course Registration
- `register_course(admin, course_id, creator, default_language, initial_metadata)`
- Creates a new course with initial metadata in the default language

### Metadata Management
- `update_course_metadata(admin, course_id, metadata)`
- Adds or updates metadata for a specific language

### Retrieval Functions
- `get_course_metadata(course_id, language_code)`
- `get_course_info(course_id)`
- `get_course_languages(course_id)`
- `get_course_registry()`

### Language Management
- `remove_course_language(admin, course_id, language_code)`
- Removes a language version (cannot remove default language)

## Usage Examples

### Register a Course
```rust
let initial_metadata = CourseMetadata {
    language_code: Symbol::new(&env, "en"),
    ipfs_link: Symbol::new(&env, "QmTest123..."),
    title: Symbol::new(&env, "Introduction to Blockchain"),
    description: Symbol::new(&env, "Learn blockchain fundamentals"),
    updated_at: 0,
};

client.register_course(&admin, &1, &creator, &Symbol::new(&env, "en"), &initial_metadata);
```

### Add Spanish Translation
```rust
let spanish_metadata = CourseMetadata {
    language_code: Symbol::new(&env, "es"),
    ipfs_link: Symbol::new(&env, "QmSpanish123..."),
    title: Symbol::new(&env, "Introducción a Blockchain"),
    description: Symbol::new(&env, "Aprende los fundamentos de blockchain"),
    updated_at: 0,
};

client.update_course_metadata(&admin, &1, &spanish_metadata);
```

## Security Features

- **Admin-Only Access**: All metadata operations require admin authorization
- **Registry Size Limits**: Prevents gas limit issues with too many courses
- **Default Language Protection**: Cannot remove the default language of a course
- **Input Validation**: Validates language codes and IPFS link formats

## Storage Architecture

- Uses Soroban persistent storage for metadata
- Efficient key structure: `CourseMetadata(course_id, language_code)`
- Maintains language index for each course
- Separates course info from language-specific metadata

## Testing

Comprehensive test suite covering:
- Course registration with metadata
- Multiple language support
- Language removal functionality
- Authorization controls
- Input validation
- Edge cases and error conditions

Run tests with:
```bash
cargo test --package scholar_contracts test_register_course_with_metadata
```

## Future Enhancements

- Enhanced IPFS CID validation
- Language-specific pricing
- Automatic translation integration
- Metadata versioning
- Batch operations for multiple languages
