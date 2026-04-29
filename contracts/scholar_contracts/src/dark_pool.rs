#![no_std]
use crate::ScholarError;
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, BytesN, Env, Map, String, Symbol, Vec,
};

// Student Profile NFT Contract for Soroban
// Implements dynamic NFTs that evolve with student achievements

use soroban_sdk::{token, Address, Env, Symbol};

use crate::{
    DataKey, ScholarContract, TuitionStipendSplit, LEDGER_BUMP_EXTEND, LEDGER_BUMP_THRESHOLD,
};

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::contractimpl;

    #[test]
    fn test_tuition_stipend_split_configuration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ScholarContract);
        let client = ScholarContractClient::new(&env, &contract_id);

        // Setup admin
        let admin = Address::generate(&env);
        client.init(&10, &3600, &10, &100, &60);
        client.set_admin(&admin);

        // Create student and university addresses
        let student = Address::generate(&env);
        let university = Address::generate(&env);

        // Configure tuition-stipend split (70% university, 30% student)
        client.set_tuition_stipend_split(
            &admin,
            &student,
            &university,
            &70, // university_percentage
            &30, // student_percentage
        );

        // Verify the configuration
        let split_config = client.get_tuition_stipend_split(&student);
        assert!(split_config.is_some());

        let config = split_config.unwrap();
        assert_eq!(config.university_address, university);
        assert_eq!(config.student_address, student);
        assert_eq!(config.university_percentage, 70);
        assert_eq!(config.student_percentage, 30);
    }

    #[test]
    fn test_tuition_stipend_split_distribution() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ScholarContract);
        let client = ScholarContractClient::new(&env, &contract_id);

        // Setup admin and token contract
        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract_id = env.register_stellar_asset_contract(token_admin.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_contract_id);

        client.init(&10, &3600, &10, &100, &60);
        client.set_admin(&admin);

        // Create addresses
        let student = Address::generate(&env);
        let university = Address::generate(&env);
        let funder = Address::generate(&env);

        // Mint tokens to funder
        token_client.mint(&funder, &1000);

        // Configure split
        client.set_tuition_stipend_split(&admin, &student, &university, &70, &30);

        // Fund scholarship (this should trigger the split)
        client.fund_scholarship(
            &funder,
            &student,
            &1000,
            &token_contract_id,
            &Symbol::new(&env, "default_roadmap"),
        );

        // Check balances - university should have 700, student scholarship should have 300
        let university_balance = token_client.balance(&university);
        let student_scholarship = client.get_scholarship(&student);

        assert_eq!(university_balance, 700);
        assert_eq!(student_scholarship.balance, 300);
    }

    #[test]
    #[should_panic(expected = "Percentages must sum to 100")]
    fn test_invalid_split_percentages() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ScholarContract);
        let client = ScholarContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let student = Address::generate(&env);
        let university = Address::generate(&env);

        client.init(&10, &3600, &10, &100, &60);
        client.set_admin(&admin);

        // Try to set invalid percentages (should panic)
        client.set_tuition_stipend_split(&admin, &student, &university, &80, &30);
        // 80 + 30 = 110
    }

    #[test]
    fn test_no_split_configuration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ScholarContract);
        let client = ScholarContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract_id = env.register_stellar_asset_contract(token_admin.clone());
        let token_client = token::StellarAssetClient::new(&env, &token_contract_id);

        client.init(&10, &3600, &10, &100, &60);
        client.set_admin(&admin);

        let student = Address::generate(&env);
        let funder = Address::generate(&env);

        token_client.mint(&funder, &1000);

        // Fund scholarship without configuring split
        client.fund_scholarship(
            &funder,
            &student,
            &1000,
            &token_contract_id,
            &Symbol::new(&env, "default_roadmap"),
        );

        // Student should receive full amount
        let student_scholarship = client.get_scholarship(&student);
        assert_eq!(student_scholarship.balance, 1000);
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StudentProfileNFT {
    pub token_id: BytesN<32>,
    pub owner: Address,
    pub student_id: String,
    pub level: u32,
    pub xp: u64,
    pub achievements: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub metadata: Map<Symbol, String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Achievement {
    pub id: String,
    pub title: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub xp_reward: u64,
    pub unlocked_at: u64,
    pub rarity: String,
}

#[contracttype]
pub enum DataKey {
    NFT(BytesN<32>),
    StudentProfile(String),
    Achievement(String),
    NextTokenId,
    LevelThreshold(u32),
    NFTCounter,
}

// Level thresholds for progression
const LEVEL_THRESHOLDS: [(u32, u64); 8] = [
    (1, 0),     // Beginner
    (2, 100),   // Novice
    (3, 250),   // Apprentice
    (4, 500),   // Scholar
    (5, 1000),  // Expert
    (6, 2000),  // Master
    (7, 5000),  // Grandmaster
    (8, 10000), // Legend
];

#[contract]
pub struct StudentProfileNFTContract;

#[contractimpl]
impl StudentProfileNFTContract {
    /// Initializes the Student Profile NFT contract with default configuration.
    ///
    /// # Input Requirements
    /// - No parameters required
    ///
    /// # Side Effects
    /// - Sets next token ID to 1 in instance storage
    /// - Initializes level thresholds for all 8 levels (Beginner to Legend)
    /// - Initializes NFT counter to 0
    ///
    /// # Level Thresholds
    /// - Level 1 (Beginner): 0 XP
    /// - Level 2 (Novice): 100 XP
    /// - Level 3 (Apprentice): 250 XP
    /// - Level 4 (Scholar): 500 XP
    /// - Level 5 (Expert): 1000 XP
    /// - Level 6 (Master): 2000 XP
    /// - Level 7 (Grandmaster): 5000 XP
    /// - Level 8 (Legend): 10000 XP
    ///
    /// # Security Considerations
    /// - Should only be called once during contract deployment
    /// - No access control - ensure this is called during deployment only
    /// - Overwrites any existing configuration if called again
    pub fn init(env: Env) {
        // Set next token ID to 1
        env.storage().instance().set(&DataKey::NextTokenId, &1u64);

        // Initialize level thresholds
        for (level, xp) in LEVEL_THRESHOLDS.iter() {
            env.storage()
                .instance()
                .set(&DataKey::LevelThreshold(*level), xp);
        }

        // Initialize NFT counter
        env.storage().instance().set(&DataKey::NFTCounter, &0u64);
    }

    /// Mints a new Student Profile NFT for a student.
    ///
    /// # Input Requirements
    /// - `owner`: The address that will own the NFT (must authenticate)
    /// - `student_id`: Unique identifier for the student (e.g., email, student number)
    /// - `initial_metadata`: Key-value pairs for initial NFT metadata (e.g., name, institution)
    ///
    /// # Access Control
    /// - Only the owner address can mint for themselves
    /// - Owner must authenticate via `require_auth()`
    ///
    /// # Returns
    /// - `BytesN<32>`: Unique 32-byte token ID for the minted NFT
    ///
    /// # Side Effects
    /// - Generates unique token ID using sequence number and timestamp
    /// - Creates StudentProfileNFT with level 1, 0 XP, empty achievements
    /// - Stores NFT data in persistent storage
    /// - Maps student_id to token_id for lookup
    /// - Increments NFT counter
    /// - Emits `NFT_Minted` event
    ///
    /// # Initial State
    /// - Level: 1 (Beginner)
    /// - XP: 0
    /// - Achievements: Empty vector
    /// - Created/Updated timestamps: Current ledger time
    ///
    /// # Security Considerations
    /// - One NFT per student_id (overwrites if exists)
    /// - Token ID generation uses timestamp for uniqueness
    /// - Owner authentication prevents unauthorized minting
    ///
    /// # Errors
    /// - Panics if owner authentication fails
    pub fn mint_nft(
        env: Env,
        owner: Address,
        student_id: String,
        initial_metadata: Map<Symbol, String>,
    ) -> BytesN<32> {
        owner.require_auth();

        // Generate unique token ID
        let next_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextTokenId)
            .unwrap_or(1);
        let token_id = Self::generate_token_id(&env, next_id);

        // Update next token ID
        env.storage()
            .instance()
            .set(&DataKey::NextTokenId, &(next_id + 1));

        // Create student profile NFT
        let nft = StudentProfileNFT {
            token_id: token_id.clone(),
            owner: owner.clone(),
            student_id: student_id.clone(),
            level: 1,
            xp: 0,
            achievements: Vec::new(&env),
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            metadata: initial_metadata,
        };

        // Store NFT data
        env.storage()
            .persistent()
            .set(&DataKey::NFT(token_id.clone()), &nft);

        // Store student profile reference
        env.storage()
            .persistent()
            .set(&DataKey::StudentProfile(student_id), &token_id);

        // Update NFT counter
        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NFTCounter)
            .unwrap_or(0);
        counter += 1;
        env.storage().instance().set(&DataKey::NFTCounter, &counter);

        // Emit mint event
        env.events().publish(
            (
                Symbol::new(&env, "NFT_Minted"),
                owner.clone(),
                token_id.clone(),
            ),
            (student_id, 1, 0),
        );

        token_id
    }

    /// Updates a student's XP and recalculates their level.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    /// - `xp_amount`: Amount of XP to add (must be >= 0)
    /// - `caller`: Must be the NFT owner (must authenticate)
    ///
    /// # Access Control
    /// - Only the NFT owner can update their own XP
    /// - Caller must authenticate via `require_auth()`
    ///
    /// # Side Effects
    /// - Adds XP amount to student's current XP
    /// - Recalculates level based on new XP total
    /// - Updates NFT timestamp
    /// - If level increases, adds level-up achievement automatically
    /// - Emits `Level_Up` event if level changes
    /// - Emits `XP_Updated` event
    ///
    /// # Level Progression
    /// Level is determined by XP thresholds:
    /// - Level increases when XP reaches next threshold
    /// - Level never decreases (XP is cumulative)
    /// - Max level is 8 (Legend) at 10000 XP
    ///
    /// # Security Considerations
    /// - XP can only be added, not subtracted
    /// - Owner-only access prevents manipulation
    /// - Level-up achievements are automatically added
    ///
    /// # Errors
    /// - Panics if caller authentication fails
    /// - Panics if caller is not the NFT owner
    /// - Panics if student profile not found
    /// - Panics if NFT not found
    pub fn update_xp(env: Env, student_id: String, xp_amount: u64, caller: Address) {
        caller.require_auth();

        // Get token ID from student profile
        let token_id: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::StudentProfile(student_id.clone()))
            .expect("Student profile not found");

        // Get current NFT data
        let mut nft: StudentProfileNFT = env
            .storage()
            .persistent()
            .get(&DataKey::NFT(token_id.clone()))
            .expect("NFT not found");

        // Verify caller is owner
        if nft.owner != caller {
            env.panic_with_error(ScholarError::OnlyOwnerCanUpdateXP);
        }

        let old_level = nft.level;
        nft.xp += xp_amount;
        nft.level = Self::calculate_level(&env, nft.xp);
        nft.updated_at = env.ledger().timestamp();

        // Store updated NFT
        env.storage()
            .persistent()
            .set(&DataKey::NFT(token_id.clone()), &nft);

        // Check for level up
        if nft.level > old_level {
            // Add level up achievement
            let achievement_title =
                format!("Level {}: {}", nft.level, Self::get_level_name(nft.level));
            nft.achievements
                .push_back(String::from_str(&env, &achievement_title));

            // Emit level up event
            env.events().publish(
                (Symbol::new(&env, "Level_Up"), caller, token_id.clone()),
                (old_level, nft.level, nft.xp),
            );
        }

        // Emit XP update event
        env.events().publish(
            (Symbol::new(&env, "XP_Updated"), caller, token_id),
            (xp_amount, nft.xp, nft.level),
        );
    }

    /// Adds an achievement to a student's profile.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    /// - `achievement`: Achievement struct containing:
    ///   - `id`: Unique achievement identifier
    ///   - `title`: Display title of achievement
    ///   - `description`: Detailed description
    ///   - `icon`: Icon identifier or URL
    ///   - `category`: Achievement category (e.g., "academic", "social")
    ///   - `xp_reward`: XP awarded for this achievement
    ///   - `unlocked_at`: Timestamp when unlocked
    ///   - `rarity`: Rarity tier (e.g., "common", "rare", "legendary")
    /// - `caller`: Must be the NFT owner (must authenticate)
    ///
    /// # Access Control
    /// - Only the NFT owner can add achievements to their profile
    /// - Caller must authenticate via `require_auth()`
    ///
    /// # Side Effects
    /// - Stores achievement in persistent storage
    /// - Adds achievement title to NFT's achievements list
    /// - Updates NFT timestamp
    /// - If achievement has XP reward, automatically calls `update_xp`
    /// - Emits `Achievement_Added` event
    ///
    /// # XP Reward
    /// - If `xp_reward > 0`, XP is automatically added to student's total
    /// - This may trigger a level-up if threshold is reached
    /// - Level-up achievement is added automatically if level changes
    ///
    /// # Security Considerations
    /// - Owner-only access prevents fake achievements
    /// - Achievement is stored separately for detailed lookup
    /// - Only title is stored in NFT for gas efficiency
    ///
    /// # Errors
    /// - Panics if caller authentication fails
    /// - Panics if caller is not the NFT owner
    /// - Panics if student profile not found
    /// - Panics if NFT not found
    pub fn add_achievement(
        env: Env,
        student_id: String,
        achievement: Achievement,
        caller: Address,
    ) {
        caller.require_auth();

        // Get token ID from student profile
        let token_id: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::StudentProfile(student_id.clone()))
            .expect("Student profile not found");

        // Get current NFT data
        let mut nft: StudentProfileNFT = env
            .storage()
            .persistent()
            .get(&DataKey::NFT(token_id.clone()))
            .expect("NFT not found");

        // Verify caller is owner
        if nft.owner != caller {
            env.panic_with_error(ScholarError::OnlyOwnerCanAddAchievements);
        }

        // Store achievement
        env.storage()
            .persistent()
            .set(&DataKey::Achievement(achievement.id.clone()), &achievement);

        // Add to NFT achievements list
        nft.achievements.push_back(achievement.title.clone());
        nft.updated_at = env.ledger().timestamp();

        // Store updated NFT
        env.storage()
            .persistent()
            .set(&DataKey::NFT(token_id), &nft);

        // Award XP if achievement has reward
        if achievement.xp_reward > 0 {
            Self::update_xp(env, student_id, achievement.xp_reward, caller);
        }

        // Emit achievement event
        env.events().publish(
            (Symbol::new(&env, "Achievement_Added"), caller, student_id),
            (achievement.title, achievement.xp_reward, achievement.rarity),
        );
    }

    /// Transfers ownership of a Student Profile NFT to a new address.
    ///
    /// # Input Requirements
    /// - `token_id`: The 32-byte token ID to transfer
    /// - `from`: Current owner address (must authenticate)
    /// - `to`: New owner address
    ///
    /// # Access Control
    /// - Only the current owner can transfer their NFT
    /// - From address must authenticate via `require_auth()`
    ///
    /// # Side Effects
    /// - Updates NFT ownership to new address
    /// - Updates NFT timestamp
    /// - Emits `NFT_Transferred` event
    ///
    /// # Security Considerations
    /// - Ownership verification prevents unauthorized transfers
    /// - Student profile mapping (student_id -> token_id) is NOT updated
    /// - This means the student_id remains associated with the token
    /// - Consider whether this is desired behavior for your use case
    ///
    /// # Errors
    /// - Panics if from address authentication fails
    /// - Panics if from address is not the current owner
    /// - Panics if NFT not found
    pub fn transfer_nft(env: Env, token_id: BytesN<32>, from: Address, to: Address) {
        from.require_auth();

        let mut nft: StudentProfileNFT = env
            .storage()
            .persistent()
            .get(&DataKey::NFT(token_id.clone()))
            .expect("NFT not found");

        // Verify from address is current owner
        if nft.owner != from {
            env.panic_with_error(ScholarError::TransferNotAuthorized);
        }

        // Update ownership
        nft.owner = to.clone();
        nft.updated_at = env.ledger().timestamp();

        // Store updated NFT
        env.storage()
            .persistent()
            .set(&DataKey::NFT(token_id), &nft);

        // Emit transfer event
        env.events()
            .publish((Symbol::new(&env, "NFT_Transferred"), from, to), token_id);
    }

    /// Retrieves NFT data by its token ID.
    ///
    /// # Input Requirements
    /// - `token_id`: The 32-byte token ID to retrieve
    ///
    /// # Returns
    /// - `StudentProfileNFT` struct containing:
    ///   - `token_id`: The NFT's unique identifier
    ///   - `owner`: Current owner address
    ///   - `student_id`: Student's unique identifier
    ///   - `level`: Current level (1-8)
    ///   - `xp`: Total XP accumulated
    ///   - `achievements`: List of achievement titles
    ///   - `created_at`: Creation timestamp
    ///   - `updated_at`: Last update timestamp
    ///   - `metadata`: Additional key-value metadata
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Errors
    /// - Panics if NFT not found
    pub fn get_nft(env: Env, token_id: BytesN<32>) -> StudentProfileNFT {
        env.storage()
            .persistent()
            .get(&DataKey::NFT(token_id))
            .expect("NFT not found")
    }

    /// Retrieves a student's NFT using their student ID.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    ///
    /// # Returns
    /// - `StudentProfileNFT` struct (see `get_nft` for details)
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Notes
    /// - This is a convenience function that looks up token_id first
    /// - More efficient if you only have student_id, not token_id
    ///
    /// # Errors
    /// - Panics if student profile not found
    /// - Panics if NFT not found
    pub fn get_nft_by_student(env: Env, student_id: String) -> StudentProfileNFT {
        let token_id: BytesN<32> = env
            .storage()
            .persistent()
            .get(&DataKey::StudentProfile(student_id))
            .expect("Student profile not found");

        Self::get_nft(env, token_id)
    }

    /// Retrieves detailed achievement data by its ID.
    ///
    /// # Input Requirements
    /// - `achievement_id`: The unique achievement identifier
    ///
    /// # Returns
    /// - `Achievement` struct containing:
    ///   - `id`: Achievement identifier
    ///   - `title`: Display title
    ///   - `description`: Detailed description
    ///   - `icon`: Icon identifier or URL
    ///   - `category`: Achievement category
    ///   - `xp_reward`: XP awarded
    ///   - `unlocked_at`: Unlock timestamp
    ///   - `rarity`: Rarity tier
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Notes
    /// - Achievement must have been added via `add_achievement`
    /// - Returns full details, not just the title stored in NFT
    ///
    /// # Errors
    /// - Panics if achievement not found
    pub fn get_achievement(env: Env, achievement_id: String) -> Achievement {
        env.storage()
            .persistent()
            .get(&DataKey::Achievement(achievement_id))
            .expect("Achievement not found")
    }

    /// Retrieves the total number of NFTs minted by the contract.
    ///
    /// # Returns
    /// - `u64`: Total count of minted NFTs
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Notes
    /// - Counter increments on each successful `mint_nft`
    /// - Used for analytics and supply tracking
    /// - Does not account for burned or transferred NFTs
    pub fn get_total_nfts(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::NFTCounter)
            .unwrap_or(0)
    }

    /// Retrieves the XP threshold required for a specific level.
    ///
    /// # Input Requirements
    /// - `level`: The level to query (1-8)
    ///
    /// # Returns
    /// - `u64`: XP required to reach this level
    /// - Returns 0 if level not found or invalid
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Level Thresholds
    /// - Level 1: 0 XP
    /// - Level 2: 100 XP
    /// - Level 3: 250 XP
    /// - Level 4: 500 XP
    /// - Level 5: 1000 XP
    /// - Level 6: 2000 XP
    /// - Level 7: 5000 XP
    /// - Level 8: 10000 XP
    ///
    /// # Notes
    /// - Useful for calculating progress to next level
    /// - Thresholds are set during contract initialization
    pub fn get_level_threshold(env: Env, level: u32) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::LevelThreshold(level))
            .unwrap_or(0)
    }

    /// Calculate level based on XP
    fn calculate_level(env: &Env, xp: u64) -> u32 {
        for (level, threshold) in LEVEL_THRESHOLDS.iter().rev() {
            if xp >= *threshold {
                return *level;
            }
        }
        1
    }

    /// Get level name
    fn get_level_name(level: u32) -> &'static str {
        match level {
            1 => "Beginner",
            2 => "Novice",
            3 => "Apprentice",
            4 => "Scholar",
            5 => "Expert",
            6 => "Master",
            7 => "Grandmaster",
            8 => "Legend",
            _ => "Unknown",
        }
    }

    /// Generate unique token ID
    fn generate_token_id(env: &Env, id: u64) -> BytesN<32> {
        let mut bytes = [0u8; 32];
        let id_bytes = id.to_be_bytes();
        let timestamp = env.ledger().timestamp().to_be_bytes();

        // Combine ID and timestamp for uniqueness
        bytes[0..8].copy_from_slice(&id_bytes);
        bytes[8..16].copy_from_slice(&timestamp[0..8]);

        // Fill remaining bytes with pseudo-random data
        for i in 16..32 {
            bytes[i] = (id + i as u64).to_be_bytes()[7];
        }

        BytesN::from_array(env, &bytes)
    }

    /// Checks if a student profile exists for the given student ID.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    ///
    /// # Returns
    /// - `true` if student profile exists, `false` otherwise
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Use Cases
    /// - Check if student has been onboarded
    /// - Prevent duplicate minting
    /// - Validate student ID before operations
    pub fn student_exists(env: Env, student_id: String) -> bool {
        env.storage()
            .persistent()
            .get::<DataKey, BytesN<32>>(&DataKey::StudentProfile(student_id))
            .is_some()
    }

    /// Retrieves a student's current level and XP total.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    ///
    /// # Returns
    /// - Tuple `(level, xp)` where:
    ///   - `level`: Current level (1-8)
    ///   - `xp`: Total XP accumulated
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Notes
    /// - Convenience function for quick level/XP lookup
    /// - More efficient than fetching full NFT if only level/XP needed
    ///
    /// # Errors
    /// - Panics if student profile not found
    pub fn get_student_level(env: Env, student_id: String) -> (u32, u64) {
        let nft = Self::get_nft_by_student(env, student_id);
        (nft.level, nft.xp)
    }

    /// Retrieves the list of achievement titles for a student.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    ///
    /// # Returns
    /// - `Vec<String>`: List of achievement titles
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Notes
    /// - Returns only achievement titles, not full details
    /// - Use `get_achievement` with achievement ID for full details
    /// - Achievements are stored in NFT for gas efficiency
    ///
    /// # Errors
    /// - Panics if student profile not found
    pub fn get_student_achievements(env: Env, student_id: String) -> Vec<String> {
        let nft = Self::get_nft_by_student(env, student_id);
        nft.achievements
    }

    /// Calculates a student's progress toward the next level.
    ///
    /// # Input Requirements
    /// - `student_id`: The student's unique identifier
    ///
    /// # Returns
    /// - Tuple `(current_xp, next_threshold, progress)` where:
    ///   - `current_xp`: Student's current XP total
    ///   - `next_threshold`: XP required for next level (0 if at max level)
    ///   - `progress`: Progress as percentage (0.0-1.0, 1.0 if at max level)
    ///
    /// # Side Effects
    /// - None (read-only function)
    ///
    /// # Progress Calculation
    /// - progress = (current_xp - current_threshold) / (next_threshold - current_threshold)
    /// - Returns 1.0 if at max level (Level 8)
    /// - Returns 0.0 if thresholds are invalid
    ///
    /// # Use Cases
    /// - Display progress bars in UI
    /// - Calculate XP needed for next level
    /// - Gamification and motivation
    ///
    /// # Errors
    /// - Panics if student profile not found
    pub fn get_level_progress(env: Env, student_id: String) -> (u64, u64, f64) {
        let nft = Self::get_nft_by_student(env, student_id);

        if nft.level >= 8 {
            return (nft.xp, 0, 1.0); // Max level
        }

        let current_threshold = Self::get_level_threshold(env, nft.level);
        let next_threshold = Self::get_level_threshold(env, nft.level + 1);
        let progress = if next_threshold > current_threshold {
            (nft.xp - current_threshold) as f64 / (next_threshold - current_threshold) as f64
        } else {
            0.0
        };

        (nft.xp, next_threshold, progress)
    }
}
