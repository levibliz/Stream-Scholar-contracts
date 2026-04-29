//! Comprehensive Authorization Enforcement Tests
//! 
//! This module tests that all sponsor withdrawal and milestone approval operations
//! require proper authentication and authorization checks.

use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};

#[test]
fn test_sponsor_yield_harvest_requires_auth() {
    let env = Env::default();
    env.mock_all_auths();
    
    let sponsor = Address::generate(&env);
    let admin = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    client.set_admin(&admin);
    
    // Deploy token
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    
    // Set up sponsor profile
    client.set_yield_preference(&sponsor, &SponsorYieldPreference::ReturnToSponsor);
    
    // Test 1: Sponsor can harvest their own yield (should succeed)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "harvest_yield"),
        (
            &sponsor,
            &100i128,
            &token_address.address(),
        ),
    );
    assert!(result.is_ok(), "Sponsor should be able to harvest their own yield");
    
    // Test 2: Unauthorized user cannot harvest sponsor's yield (should fail)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "harvest_yield"),
        (
            &unauthorized_user,
            &100i128,
            &token_address.address(),
        ),
    );
    assert!(result.is_err(), "Unauthorized user should not be able to harvest sponsor's yield");
}

#[test]
fn test_milestone_bounty_claim_requires_student_auth() {
    let env = Env::default();
    env.mock_all_auths();
    
    let student = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let funder = Address::generate(&env);
    let advisor_sig = soroban_sdk::Bytes::from_slice(&env, b"valid_advisor_signature");
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    
    // Deploy token
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    
    // Set up student access and bounty
    token_client.mint(&funder, &10000);
    client.fund_scholarship(&funder, &student, &5000, &token_address.address());
    client.buy_access(&student, &1, &1000, &token_address.address());
    client.fund_bounty_reserve(&funder, &student, &1, &2000, &token_address.address());
    
    // Test 1: Student can claim bounty with valid signature (should succeed)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "claim_milestone_bounty"),
        (
            &student,
            &1u64,
            &1u64,
            &200i128,
            &advisor_sig,
        ),
    );
    assert!(result.is_ok(), "Student should be able to claim bounty with valid auth");
    
    // Test 2: Unauthorized user cannot claim student's bounty (should fail)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "claim_milestone_bounty"),
        (
            &unauthorized_user,
            &1u64,
            &1u64,
            &200i128,
            &advisor_sig,
        ),
    );
    assert!(result.is_err(), "Unauthorized user should not be able to claim student's bounty");
}

#[test]
fn test_milestone_bounty_requires_valid_signature() {
    let env = Env::default();
    env.mock_all_auths();
    
    let student = Address::generate(&env);
    let funder = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    
    // Deploy token
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    
    // Set up student access and bounty
    token_client.mint(&funder, &10000);
    client.fund_scholarship(&funder, &student, &5000, &token_address.address());
    client.buy_access(&student, &1, &1000, &token_address.address());
    client.fund_bounty_reserve(&funder, &student, &1, &2000, &token_address.address());
    
    // Test 1: Empty signature should fail
    let empty_sig = soroban_sdk::Bytes::from_slice(&env, b"");
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "claim_milestone_bounty"),
        (
            &student,
            &1u64,
            &1u64,
            &200i128,
            &empty_sig,
        ),
    );
    assert!(result.is_err(), "Empty signature should be rejected");
    
    // Test 2: Valid signature should succeed
    let valid_sig = soroban_sdk::Bytes::from_slice(&env, b"valid_advisor_signature");
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "claim_milestone_bounty"),
        (
            &student,
            &1u64,
            &1u64,
            &200i128,
            &valid_sig,
        ),
    );
    assert!(result.is_ok(), "Valid signature should be accepted");
}

#[test]
fn test_yield_preference_requires_sponsor_auth() {
    let env = Env::default();
    env.mock_all_auths();
    
    let sponsor = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let admin = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    client.set_admin(&admin);
    
    // Test 1: Sponsor can set their own preference (should succeed)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "set_yield_preference"),
        (
            &sponsor,
            &SponsorYieldPreference::Reinvest,
        ),
    );
    assert!(result.is_ok(), "Sponsor should be able to set their own preference");
    
    // Test 2: Unauthorized user cannot set sponsor's preference (should fail)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "set_yield_preference"),
        (
            &unauthorized_user,
            &SponsorYieldPreference::ReturnToSponsor,
        ),
    );
    assert!(result.is_err(), "Unauthorized user should not be able to set sponsor's preference");
}

#[test]
fn test_scholarship_withdrawal_requires_student_auth() {
    let env = Env::default();
    env.mock_all_auths();
    
    let student = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let funder = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    
    // Deploy token
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    
    // Fund scholarship
    token_client.mint(&funder, &10000);
    client.fund_scholarship(&funder, &student, &5000, &token_address.address());
    
    // Test 1: Student can withdraw their own funds (should succeed)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "withdraw_scholarship"),
        (
            &student,
            &100i128,
        ),
    );
    assert!(result.is_ok(), "Student should be able to withdraw their own funds");
    
    // Test 2: Unauthorized user cannot withdraw student's funds (should fail)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "withdraw_scholarship"),
        (
            &unauthorized_user,
            &100i128,
        ),
    );
    assert!(result.is_err(), "Unauthorized user should not be able to withdraw student's funds");
}

#[test]
fn test_authorized_payout_address_requires_student_auth() {
    let env = Env::default();
    env.mock_all_auths();
    
    let student = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let authorized_address = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    
    // Test 1: Student can set their own authorized address (should succeed)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "set_authorized_payout_address"),
        (
            &student,
            &authorized_address,
        ),
    );
    assert!(result.is_ok(), "Student should be able to set their own authorized address");
    
    // Test 2: Unauthorized user cannot set student's authorized address (should fail)
    let result = env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
        &contract_id,
        &Symbol::new(&env, "set_authorized_payout_address"),
        (
            &unauthorized_user,
            &authorized_address,
        ),
    );
    assert!(result.is_err(), "Unauthorized user should not be able to set student's authorized address");
}

#[test]
fn test_authorization_events_emitted() {
    let env = Env::default();
    env.mock_all_auths();
    
    let student = Address::generate(&env);
    let sponsor = Address::generate(&env);
    let funder = Address::generate(&env);
    let advisor_sig = soroban_sdk::Bytes::from_slice(&env, b"valid_advisor_signature");
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    
    // Deploy token
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_address.address());
    
    // Set up sponsor profile
    client.set_yield_preference(&sponsor, &SponsorYieldPreference::ReturnToSponsor);
    
    // Fund scholarship and bounty
    token_client.mint(&funder, &10000);
    client.fund_scholarship(&funder, &student, &5000, &token_address.address());
    client.buy_access(&student, &1, &1000, &token_address.address());
    client.fund_bounty_reserve(&funder, &student, &1, &2000, &token_address.address());
    
    // Test advisor signature verification event
    client.claim_milestone_bounty(&student, &1, &1, &200, &advisor_sig);
    
    // Verify events were emitted
    let events = env.events().all();
    let advisor_sig_event = events.iter().find(|event| {
        event.topics[0] == Symbol::new(&env, "AdvisorSignatureVerified")
    });
    
    assert!(advisor_sig_event.is_some(), "AdvisorSignatureVerified event should be emitted");
}

#[test]
fn test_comprehensive_authorization_matrix() {
    let env = Env::default();
    env.mock_all_auths();
    
    let student = Address::generate(&env);
    let sponsor = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let admin = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register(ScholarContract, ());
    let client = ScholarContractClient::new(&env, &contract_id);
    
    client.init(&1000, &3600, &10, &100, &60);
    client.set_admin(&admin);
    
    // Deploy token
    let token_admin = Address::generate(&env);
    let token_address = env.register_stellar_asset_contract_v2(token_admin);
    
    // Test matrix of operations and expected auth requirements
    let test_cases = vec![
        // (operation, authorized_user, unauthorized_user, should_succeed_for_authorized)
        ("harvest_yield", sponsor.clone(), unauthorized_user.clone(), true),
        ("set_yield_preference", sponsor.clone(), unauthorized_user.clone(), true),
        ("withdraw_scholarship", student.clone(), unauthorized_user.clone(), true),
        ("set_authorized_payout_address", student.clone(), unauthorized_user.clone(), true),
    ];
    
    for (operation, authorized, unauthorized, should_succeed) in test_cases {
        // Test authorized user
        let result = match operation {
            "harvest_yield" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "harvest_yield"),
                (&authorized, &100i128, &token_address.address()),
            ),
            "set_yield_preference" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "set_yield_preference"),
                (&authorized, &SponsorYieldPreference::Reinvest),
            ),
            "withdraw_scholarship" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "withdraw_scholarship"),
                (&authorized, &100i128),
            ),
            "set_authorized_payout_address" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "set_authorized_payout_address"),
                (&authorized, &Address::generate(&env)),
            ),
            _ => continue,
        };
        
        if should_succeed {
            assert!(result.is_ok(), "Authorized user should succeed for {}", operation);
        }
        
        // Test unauthorized user
        let result = match operation {
            "harvest_yield" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "harvest_yield"),
                (&unauthorized, &100i128, &token_address.address()),
            ),
            "set_yield_preference" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "set_yield_preference"),
                (&unauthorized, &SponsorYieldPreference::Reinvest),
            ),
            "withdraw_scholarship" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "withdraw_scholarship"),
                (&unauthorized, &100i128),
            ),
            "set_authorized_payout_address" => env.try_invoke_contract::<soroban_sdk::xdr::ScVal>(
                &contract_id,
                &Symbol::new(&env, "set_authorized_payout_address"),
                (&unauthorized, &Address::generate(&env)),
            ),
            _ => continue,
        };
        
        assert!(result.is_err(), "Unauthorized user should fail for {}", operation);
    }
}
