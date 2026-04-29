// Issues #231–#234 — second contractimpl block for ScholarContract.

#[contractimpl]
impl ScholarContract {
    pub fn set_reputation_fee_sink(env: Env, auth: Address, sink: Address) {
        auth.require_auth();
        let council: Address = env
            .storage()
            .instance()
            .get(&DataKey::SecurityCouncil)
            .unwrap_or_else(|| panic!("SecurityCouncil not configured"));
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"));
        if auth != council && auth != admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::ReputationFeeSink, &sink);
    }

    pub fn clear_export_discipline_hold(env: Env, council: Address, student: Address) {
        council.require_auth();
        let sec: Address = env
            .storage()
            .instance()
            .get(&DataKey::SecurityCouncil)
            .expect("SecurityCouncil not configured");
        if council != sec {
            panic!("Unauthorized");
        }
        env.storage()
            .persistent()
            .remove(&DataKey::ExportDisciplineHold(student));
    }

    pub fn request_reputation_export(
        env: Env,
        student: Address,
        fee_token: Address,
        client_nonce: u64,
    ) -> u64 {
        student.require_auth();
        if env
            .storage()
            .persistent()
            .get::<_, bool>(&DataKey::ExportDisciplineHold(student.clone()))
            .unwrap_or(false)
        {
            panic!("ExportBlockedPendingDiscipline");
        }
        let nonce_key = DataKey::ReputationExportNonce(student.clone(), client_nonce);
        if env.storage().persistent().has(&nonce_key) {
            panic!("ReputationExportReplay");
        }
        env.storage().persistent().set(&nonce_key, &true);

        let fee_sink: Address = env
            .storage()
            .instance()
            .get(&DataKey::ReputationFeeSink)
            .unwrap_or_else(|| env.current_contract_address());
        let fee_client = token::Client::new(&env, &fee_token);
        fee_client.transfer(&student, &fee_sink, &REPUTATION_EXPORT_FEE_STROOPS);

        let sch_d = Self::digest_scholarship_history(&env, student.clone());
        let gpa_d = Self::digest_gpa_status(&env, student.clone());
        let grad_d = Self::digest_graduation(&env, student.clone());

        let mut preimage = soroban_sdk::Bytes::new(&env);
        let tag = b"StreamScholarReputationExport/v1";
        for i in 0..tag.len() {
            preimage.push_back(tag[i]);
        }
        for b in client_nonce.to_be_bytes() {
            preimage.push_back(b);
        }
        for b in sch_d.to_array() {
            preimage.push_back(b);
        }
        for b in gpa_d.to_array() {
            preimage.push_back(b);
        }
        for b in grad_d.to_array() {
            preimage.push_back(b);
        }
        let payload_hash_h = env.crypto().sha256(&preimage);
        let payload_bn: BytesN<32> = payload_hash_h.clone().into();

        let seq_prev: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ReputationExportSequence)
            .unwrap_or(0);
        let seq = crate::safe_math::add_u64(&env, seq_prev, 1);
        env.storage()
            .instance()
            .set(&DataKey::ReputationExportSequence, &seq);

        let mut wire = soroban_sdk::Bytes::new(&env);
        let wh = b"wormhole_vaax1";
        for i in 0..wh.len() {
            wire.push_back(wh[i]);
        }
        for b in seq.to_be_bytes() {
            wire.push_back(b);
        }
        for b in payload_bn.to_array() {
            wire.push_back(b);
        }
        let msg_id_h = env.crypto().sha256(&wire);
        let msg_bn: BytesN<32> = msg_id_h.clone().into();
        let dedup_key = DataKey::ReputationExportDedup(msg_bn.clone());
        if env.storage().persistent().has(&dedup_key) {
            panic!("CrossChainDedupCollision");
        }
        env.storage().persistent().set(&dedup_key, &true);

        let audit: ScholarExportAudit = env
            .storage()
            .persistent()
            .get(&DataKey::ExportedScholarAudit(student.clone()))
            .unwrap_or(ScholarExportAudit {
                export_count: 0,
                last_seq: 0,
            });
        let audit = ScholarExportAudit {
            export_count: audit.export_count.saturating_add(1),
            last_seq: seq,
        };
        env.storage()
            .persistent()
            .set(&DataKey::ExportedScholarAudit(student.clone()), &audit);

        env.storage().temporary().set(
            &DataKey::ReputationExportTemp(seq),
            &TempReputationExportMeta {
                student: student.clone(),
                payload_hash: payload_bn.clone(),
                ledger_time: env.ledger().timestamp(),
            },
        );

        #[allow(deprecated)]
        env.events().publish(
            (
                Symbol::new(&env, "WormholeReputationExport"),
                student.clone(),
                seq,
            ),
            (payload_bn, msg_bn),
        );
        seq
    }

    fn digest_scholarship_history(env: &Env, student: Address) -> BytesN<32> {
        let sch: Scholarship = env
            .storage()
            .persistent()
            .get(&DataKey::Scholarship(student.clone()))
            .unwrap_or(Scholarship {
                funder: student.clone(),
                balance: 0,
                token: student.clone(),
                unlocked_balance: 0,
                last_verif: 0,
                is_paused: false,
                is_disputed: false,
                dispute_reason: None,
                final_ruling: None,
                is_native: false,
                total_grant: 0,
                final_release_claimed: false,
            });
        let mut p = soroban_sdk::Bytes::new(env);
        let tag = b"SCHOL_HIST";
        for i in 0..tag.len() {
            p.push_back(tag[i]);
        }
        for b in sch.balance.to_be_bytes() {
            p.push_back(b);
        }
        for b in sch.unlocked_balance.to_be_bytes() {
            p.push_back(b);
        }
        for b in sch.total_grant.to_be_bytes() {
            p.push_back(b);
        }
        env.crypto().sha256(&p).into()
    }

    fn digest_gpa_status(env: &Env, student: Address) -> BytesN<32> {
        let gpa = env
            .storage()
            .persistent()
            .get::<_, StudentGPA>(&DataKey::StudentGPA(student.clone()))
            .unwrap_or(StudentGPA { gpa: 0, last_updated: 0, oracle_verified: false });
        let mut p = soroban_sdk::Bytes::new(env);
        let tag = b"GPA_DIG";
        for i in 0..tag.len() {
            p.push_back(tag[i]);
        }
        for b in gpa.gpa.to_be_bytes() {
            p.push_back(b);
        }
        env.crypto().sha256(&p).into()
    }

    fn digest_graduation(env: &Env, student: Address) -> BytesN<32> {
        let gp: Option<GraduateProfile> = env
            .storage()
            .persistent()
            .get(&DataKey::GraduationRegistry(student.clone()));
        let mut p = soroban_sdk::Bytes::new(env);
        let tag = b"GRAD_DIG";
        for i in 0..tag.len() {
            p.push_back(tag[i]);
        }
        if let Some(prof) = gp {
            for b in prof.graduation_date.to_be_bytes() {
                p.push_back(b);
            }
            for b in prof.final_gpa.to_be_bytes() {
                p.push_back(b);
            }
            p.push_back(prof.completed_scholarships.len() as u8);
        } else {
            p.push_back(0);
        }
        env.crypto().sha256(&p).into()
    }

    pub fn export_audit(env: Env, student: Address) -> ScholarExportAudit {
        env.storage()
            .persistent()
            .get(&DataKey::ExportedScholarAudit(student))
            .unwrap_or(ScholarExportAudit {
                export_count: 0,
                last_seq: 0,
            })
    }

    pub fn reputation_export_sequence(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::ReputationExportSequence)
            .unwrap_or(0)
    }

    pub fn init_grant_milestone_deps(
        env: Env,
        authority: Address,
        student: Address,
        course_id: u64,
        cfg: GrantMilestoneConfig,
    ) {
        authority.require_auth();
        if !Self::is_admin(&env, &authority)
            && !env
                .storage()
                .instance()
                .get::<_, bool>(&DataKey::OracleStatus(authority.clone()))
                .unwrap_or(false)
        {
            panic!("Unauthorized");
        }
        if cfg.milestone_count == 0 || cfg.milestone_count > MAX_MILESTONE_SLOTS {
            panic!("BadMilestoneCount");
        }
        if cfg.parent_masks.len() != cfg.milestone_count {
            panic!("BadMaskLen");
        }
        if crate::issue_features::milestone_graph_has_cycle(cfg.milestone_count, &cfg.parent_masks) {
            panic!("MilestoneDependencyCycle");
        }
        env.storage()
            .persistent()
            .set(&DataKey::GrantMilestoneParents(student, course_id), &cfg);
    }

    pub fn revoke_milestone_parent(
        env: Env,
        oracle: Address,
        student: Address,
        course_id: u64,
        revoked_idx: u32,
    ) {
        oracle.require_auth();
        Self::verify_oracle_authorization(&env, &oracle);
        let cfg: GrantMilestoneConfig = env
            .storage()
            .persistent()
            .get(&DataKey::GrantMilestoneParents(student.clone(), course_id))
            .expect("GrantMilestoneParents not configured");
        let desc = crate::issue_features::descendant_mask(cfg.milestone_count, &cfg.parent_masks, revoked_idx);
        for i in 0..cfg.milestone_count {
            if (desc >> i) & 1 == 1 {
                env.storage().persistent().set(
                    &DataKey::MilestoneFrozen(student.clone(), course_id, i as u64),
                    &true,
                );
            }
        }
        env.storage().persistent().set(
            &DataKey::MilestoneRevoked(student.clone(), course_id, revoked_idx as u64),
            &true,
        );
    }

    pub fn set_institutional_match_cap(env: Env, authority: Address, school: Address, cap: u128) {
        authority.require_auth();
        let council: Address = env
            .storage()
            .instance()
            .get(&DataKey::SecurityCouncil)
            .unwrap_or_else(|| panic!("SecurityCouncil not configured"));
        if authority != council && !Self::is_admin(&env, &authority) {
            panic!("Unauthorized");
        }
        env.storage()
            .persistent()
            .set(&DataKey::InstitutionalPeriodicCap(school), &cap);
    }

    pub fn get_institutional_state(env: Env, school: Address) -> InstitutionalState {
        env.storage()
            .persistent()
            .get(&DataKey::InstitutionalMatchTotal(school.clone()))
            .unwrap_or(InstitutionalState {
                total_matched_volume: 0,
                last_updated: 0,
            })
    }

    pub fn configure_milestone_committee(
        env: Env,
        admin: Address,
        student: Address,
        course_id: u64,
        committee: MilestoneReviewCommittee,
    ) {
        admin.require_auth();
        if !Self::is_admin(&env, &admin) {
            panic!("Unauthorized");
        }
        env.storage()
            .persistent()
            .set(&DataKey::GrantReviewerCommittee(student, course_id), &committee);
    }

    pub fn register_committee_member(env: Env, admin: Address, committee_id: u64, member: Address) {
        admin.require_auth();
        if !Self::is_admin(&env, &admin) {
            panic!("Unauthorized");
        }
        let idx: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CommitteeNextMemberIdx(committee_id))
            .unwrap_or(0);
        if idx >= 64 {
            panic!("CommitteeFull");
        }
        env.storage()
            .persistent()
            .set(&DataKey::CommitteeMember(committee_id, member.clone()), &true);
        env.storage()
            .persistent()
            .set(&DataKey::CommitteeMemberSlot(committee_id, member.clone()), &idx);
        env.storage().instance().set(
            &DataKey::CommitteeNextMemberIdx(committee_id),
            &crate::safe_math::add_u32(&env, idx, 1),
        );
    }

    pub fn mark_committee_sep12_verified(env: Env, admin: Address, member: Address, verified: bool) {
        admin.require_auth();
        if !Self::is_admin(&env, &admin) {
            panic!("Unauthorized");
        }
        env.storage()
            .persistent()
            .set(&DataKey::CommitteeSep12Verified(member), &verified);
    }

    pub fn committee_sign_milestone(
        env: Env,
        signer: Address,
        student: Address,
        course_id: u64,
        milestone_id: u64,
    ) {
        signer.require_auth();
        let cfg: MilestoneReviewCommittee = env
            .storage()
            .persistent()
            .get(&DataKey::GrantReviewerCommittee(student.clone(), course_id))
            .expect("Committee not configured");
        let mem_ok: bool = env
            .storage()
            .persistent()
            .get(&DataKey::CommitteeMember(cfg.committee_id, signer.clone()))
            .unwrap_or(false);
        if !mem_ok {
            panic!("NotCommitteeMember");
        }
        let sep: bool = env
            .storage()
            .persistent()
            .get(&DataKey::CommitteeSep12Verified(signer.clone()))
            .unwrap_or(false);
        if !sep {
            panic!("Sep12Required");
        }
        let slot: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::CommitteeMemberSlot(cfg.committee_id, signer.clone()))
            .expect("CommitteeMemberSlot missing");
        if slot >= 64 {
            panic!("BadSlot");
        }

        let mut session: MilestoneReviewSession = env
            .storage()
            .persistent()
            .get(&DataKey::MilestoneReviewSession(
                student.clone(),
                course_id,
                milestone_id,
            ))
            .unwrap_or(MilestoneReviewSession {
                started_at: 0,
                finalized: false,
            });
        if session.finalized {
            panic!("AlreadyFinalized");
        }
        let now = env.ledger().timestamp();
        if session.started_at == 0 {
            session.started_at = now;
        }
        env.storage().persistent().set(
            &DataKey::MilestoneReviewSession(student.clone(), course_id, milestone_id),
            &session,
        );

        let mut bmp: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::CommitteeApprovalBitmap(
                student.clone(),
                course_id,
                milestone_id,
            ))
            .unwrap_or(0);
        bmp |= 1u64 << slot;
        env.storage().persistent().set(
            &DataKey::CommitteeApprovalBitmap(student.clone(), course_id, milestone_id),
            &bmp,
        );

        #[allow(deprecated)]
        env.events().publish(
            (Symbol::new(&env, "CommitteeReviewStarted"), student.clone(), milestone_id),
            cfg.committee_id,
        );

        let pop = Self::popcount64(bmp);
        if pop >= cfg.approval_threshold {
            session.finalized = true;
            env.storage().persistent().set(
                &DataKey::MilestoneReviewSession(student.clone(), course_id, milestone_id),
                &session,
            );
            #[allow(deprecated)]
            env.events().publish(
                (
                    Symbol::new(&env, "CommitteeReviewFinalized"),
                    student.clone(),
                    milestone_id,
                ),
                cfg.committee_id,
            );
        }
    }

    pub fn sudo_finalize_committee_review(
        env: Env,
        council: Address,
        student: Address,
        course_id: u64,
        milestone_id: u64,
    ) {
        council.require_auth();
        let sec: Address = env
            .storage()
            .instance()
            .get(&DataKey::SecurityCouncil)
            .expect("SecurityCouncil not configured");
        if council != sec {
            panic!("Unauthorized");
        }
        let session: MilestoneReviewSession = env
            .storage()
            .persistent()
            .get(&DataKey::MilestoneReviewSession(
                student.clone(),
                course_id,
                milestone_id,
            ))
            .expect("No session");
        if session.started_at == 0 {
            panic!("NoActiveCommitteeSession");
        }
        if env.ledger().timestamp() < session.started_at.saturating_add(COMMITTEE_REVIEW_TIMEOUT_SECS)
        {
            panic!("CommitteeStillInWindow");
        }
        let mut session = session;
        session.finalized = true;
        env.storage().persistent().set(
            &DataKey::MilestoneReviewSession(student.clone(), course_id, milestone_id),
            &session,
        );
        #[allow(deprecated)]
        env.events().publish(
            (
                Symbol::new(&env, "CommitteeReviewFinalized"),
                student.clone(),
                milestone_id,
            ),
            Symbol::new(&env, "sudo"),
        );
    }

    fn popcount64(mut x: u64) -> u32 {
        let mut c = 0u32;
        while x > 0 {
            c += (x & 1) as u32;
            x >>= 1;
        }
        c
    }

    fn milestone_prereqs_satisfied(
        env: &Env,
        student: Address,
        course_id: u64,
        milestone_id: u64,
        cfg: &GrantMilestoneConfig,
    ) -> bool {
        if milestone_id >= cfg.milestone_count as u64 {
            return false;
        }
        let mask = cfg.parent_masks.get(milestone_id as u32).unwrap_or(0);
        for p in 0..cfg.milestone_count {
            if (mask >> p) & 1 == 0 {
                continue;
            }
            let ck = DataKey::ClaimedMilestone(student.clone(), course_id, p as u64);
            if !env.storage().persistent().has(&ck) {
                return false;
            }
        }
        true
    }

    fn emit_milestone_ready_events(
        env: Env,
        student: Address,
        course_id: u64,
        cfg: GrantMilestoneConfig,
        newly_completed: u64,
    ) {
        let m = newly_completed as u32;
        if m >= cfg.milestone_count {
            return;
        }
        for child in 0..cfg.milestone_count {
            if child == m {
                continue;
            }
            let cm = cfg.parent_masks.get(child as u32).unwrap_or(0);
            if (cm >> m) & 1 == 0 {
                continue;
            }
            let mut ok = true;
            for p in 0..cfg.milestone_count {
                if p == m {
                    continue;
                }
                if (cm >> p) & 1 == 0 {
                    continue;
                }
                let ck = DataKey::ClaimedMilestone(student.clone(), course_id, p as u64);
                if !env.storage().persistent().has(&ck) {
                    ok = false;
                    break;
                }
            }
            if ok {
                #[allow(deprecated)]
                env.events().publish(
                    (
                        Symbol::new(&env, "MilestoneReady"),
                        student.clone(),
                        course_id,
                    ),
                    child as u64,
                );
            }
        }
    }

}
