use crate::common::*;
use crate::runtime::{endpoint::*, *};

#[derive(Debug)]
pub(crate) struct MonoN {
    pub ekeys: HashSet<Key>,
    pub result: Option<(usize, HashMap<Key, Payload>)>,
}
#[derive(Debug)]
pub(crate) struct PolyN {
    pub ekeys: HashSet<Key>,
    pub branches: HashMap<Predicate, BranchN>,
}
#[derive(Debug, Clone)]
pub(crate) struct BranchN {
    pub to_get: HashSet<Key>,
    pub gotten: HashMap<Key, Payload>,
    pub sync_batch_index: usize,
}

#[derive(Debug)]
pub struct MonoP {
    pub state: ProtocolS,
    pub ekeys: HashSet<Key>,
}
#[derive(Debug)]
pub(crate) struct PolyP {
    pub incomplete: HashMap<Predicate, BranchP>,
    pub complete: HashMap<Predicate, BranchP>,
    pub ekeys: HashSet<Key>,
}
#[derive(Debug, Clone)]
pub(crate) struct BranchP {
    pub outbox: HashMap<Key, Payload>,
    pub inbox: HashMap<Key, Payload>,
    pub state: ProtocolS,
}

//////////////////////////////////////////////////////////////////

impl PolyP {
    pub(crate) fn poly_run(
        &mut self,
        m_ctx: PolyPContext,
        protocol_description: &ProtocolD,
    ) -> Result<SyncRunResult, EndpointErr> {
        let to_run: Vec<_> = self.incomplete.drain().collect();
        self.poly_run_these_branches(m_ctx, protocol_description, to_run)
    }

    pub(crate) fn poly_run_these_branches(
        &mut self,
        mut m_ctx: PolyPContext,
        protocol_description: &ProtocolD,
        mut to_run: Vec<(Predicate, BranchP)>,
    ) -> Result<SyncRunResult, EndpointErr> {
        use SyncRunResult as Srr;
        log!(&mut m_ctx.inner.logger, "~ Running branches for PolyP {:?}!", m_ctx.my_subtree_id,);
        'to_run_loop: while let Some((mut predicate, mut branch)) = to_run.pop() {
            let mut r_ctx = BranchPContext {
                m_ctx: m_ctx.reborrow(),
                ekeys: &self.ekeys,
                predicate: &predicate,
                inbox: &branch.inbox,
            };
            use PolyBlocker as Sb;
            let blocker = branch.state.sync_run(&mut r_ctx, protocol_description);
            log!(
                &mut r_ctx.m_ctx.inner.logger,
                "~ ... ran PolyP {:?} with branch pred {:?} to blocker {:?}",
                r_ctx.m_ctx.my_subtree_id,
                &predicate,
                &blocker
            );
            match blocker {
                Sb::Inconsistent => {} // DROP
                Sb::CouldntReadMsg(ekey) => {
                    assert!(self.ekeys.contains(&ekey));
                    let channel_id =
                        r_ctx.m_ctx.inner.endpoint_exts.get(ekey).unwrap().info.channel_id;
                    log!(
                        &mut r_ctx.m_ctx.inner.logger,
                        "~ ... {:?} couldnt read msg for port {:?}. has inbox {:?}",
                        r_ctx.m_ctx.my_subtree_id,
                        channel_id,
                        &branch.inbox,
                    );
                    if predicate.replace_assignment(channel_id, true) != Some(false) {
                        // don't rerun now. Rerun at next `sync_run`

                        log!(&mut m_ctx.inner.logger, "~ ... Delay {:?}", m_ctx.my_subtree_id,);
                        self.incomplete.insert(predicate, branch);
                    } else {
                        log!(&mut m_ctx.inner.logger, "~ ... Drop {:?}", m_ctx.my_subtree_id,);
                    }
                    // ELSE DROP
                }
                Sb::CouldntCheckFiring(ekey) => {
                    assert!(self.ekeys.contains(&ekey));
                    let channel_id =
                        r_ctx.m_ctx.inner.endpoint_exts.get(ekey).unwrap().info.channel_id;
                    // split the branch!
                    let branch_f = branch.clone();
                    let mut predicate_f = predicate.clone();
                    if predicate_f.replace_assignment(channel_id, false).is_some() {
                        panic!("OI HANS QUERY FIRST!");
                    }
                    assert!(predicate.replace_assignment(channel_id, true).is_none());
                    to_run.push((predicate, branch));
                    to_run.push((predicate_f, branch_f));
                }
                Sb::SyncBlockEnd => {
                    let ControllerInner { logger, endpoint_exts, .. } = m_ctx.inner;
                    log!(
                        logger,
                        "~ ... ran {:?} reached SyncBlockEnd with pred {:?} ...",
                        m_ctx.my_subtree_id,
                        &predicate,
                    );
                    // come up with the predicate for this local solution

                    for ekey in self.ekeys.iter() {
                        let channel_id = endpoint_exts.get(*ekey).unwrap().info.channel_id;
                        let fired =
                            branch.inbox.contains_key(ekey) || branch.outbox.contains_key(ekey);
                        match predicate.query(channel_id) {
                            Some(true) => {
                                if !fired {
                                    // This branch should have fired but didn't!
                                    log!(
                                        logger,
                                        "~ ... ... should have fired {:?} and didn't! pruning!",
                                        channel_id,
                                    );
                                    continue 'to_run_loop;
                                }
                            }
                            Some(false) => assert!(!fired),
                            None => {
                                predicate.replace_assignment(channel_id, false);
                                assert!(!fired)
                            }
                        }
                    }
                    log!(logger, "~ ... ... and finished just fine!",);
                    m_ctx.solution_storage.submit_and_digest_subtree_solution(
                        &mut m_ctx.inner.logger,
                        m_ctx.my_subtree_id,
                        predicate.clone(),
                    );
                    self.complete.insert(predicate, branch);
                }
                Sb::PutMsg(ekey, payload) => {
                    assert!(self.ekeys.contains(&ekey));
                    let EndpointExt { info, endpoint } =
                        m_ctx.inner.endpoint_exts.get_mut(ekey).unwrap();
                    if predicate.replace_assignment(info.channel_id, true) != Some(false) {
                        branch.outbox.insert(ekey, payload.clone());
                        let msg = CommMsgContents::SendPayload {
                            payload_predicate: predicate.clone(),
                            payload,
                        }
                        .into_msg(m_ctx.inner.round_index);
                        endpoint.send(msg)?;
                        to_run.push((predicate, branch));
                    }
                    // ELSE DROP
                }
            }
        }
        // all in self.incomplete most recently returned Blocker::CouldntReadMsg
        Ok(if self.incomplete.is_empty() {
            if self.complete.is_empty() {
                Srr::NoBranches
            } else {
                Srr::AllBranchesComplete
            }
        } else {
            Srr::BlockingForRecv
        })
    }

    pub(crate) fn poly_recv_run(
        &mut self,
        m_ctx: PolyPContext,
        protocol_description: &ProtocolD,
        ekey: Key,
        payload_predicate: Predicate,
        payload: Payload,
    ) -> Result<SyncRunResult, EndpointErr> {
        // try exact match

        let to_run = if self.complete.contains_key(&payload_predicate) {
            // exact match with stopped machine

            log!(
                &mut m_ctx.inner.logger,
                "... poly_recv_run matched stopped machine exactly! nothing to do here",
            );
            vec![]
        } else if let Some(mut branch) = self.incomplete.remove(&payload_predicate) {
            // exact match with running machine

            log!(
                &mut m_ctx.inner.logger,
                "... poly_recv_run matched running machine exactly! pred is {:?}",
                &payload_predicate
            );
            branch.inbox.insert(ekey, payload);
            vec![(payload_predicate, branch)]
        } else {
            log!(
                &mut m_ctx.inner.logger,
                "... poly_recv_run didn't have any exact matches... Let's try feed it to all branches",

            );
            let mut incomplete2 = HashMap::<_, _>::default();
            let to_run = self
                .incomplete
                .drain()
                .filter_map(|(old_predicate, mut branch)| {
                    use CommonSatResult as Csr;
                    match old_predicate.common_satisfier(&payload_predicate) {
                        Csr::FormerNotLatter | Csr::Equivalent => {
                            log!(
                                &mut m_ctx.inner.logger,
                                "... poly_recv_run This branch is compatible unaltered! branch pred: {:?}",
                                &old_predicate
                            );
                            // old_predicate COVERS the assumptions of payload_predicate
                            let was = branch.inbox.insert(ekey, payload.clone());
                            assert!(was.is_none()); // INBOX MUST BE EMPTY!
                            Some((old_predicate, branch))
                        }
                        Csr::New(new) => {
                            log!(
                                &mut m_ctx.inner.logger,
                                "... poly_recv_run payloadpred {:?} and branchpred {:?} satisfied by new pred {:?}. FORKING",
                                &payload_predicate,
                                &old_predicate,
                                &new,
                            );
                            // payload_predicate has new assumptions. FORK!
                            let mut payload_branch = branch.clone();
                            let was = payload_branch.inbox.insert(ekey, payload.clone());
                            assert!(was.is_none()); // INBOX MUST BE EMPTY!

                            // put the original back untouched
                            incomplete2.insert(old_predicate, branch);
                            Some((new, payload_branch))
                        }
                        Csr::LatterNotFormer => {
                            log!(
                                &mut m_ctx.inner.logger,
                                "... poly_recv_run payloadpred {:?} subsumes branch pred {:?}. FORKING",
                                &old_predicate,
                                &payload_predicate,
                            );
                            // payload_predicate has new assumptions. FORK!
                            let mut payload_branch = branch.clone();
                            let was = payload_branch.inbox.insert(ekey, payload.clone());
                            assert!(was.is_none()); // INBOX MUST BE EMPTY!

                            // put the original back untouched
                            incomplete2.insert(old_predicate, branch);
                            Some((payload_predicate.clone(), payload_branch))
                        }
                        Csr::Nonexistant => {
                            log!(
                                &mut m_ctx.inner.logger,
                                "... poly_recv_run SKIPPING because branchpred={:?}. payloadpred={:?}",
                                &old_predicate,
                                &payload_predicate,
                            );
                            // predicates contradict
                            incomplete2.insert(old_predicate, branch);
                            None
                        }
                    }
                })
                .collect();
            std::mem::swap(&mut self.incomplete, &mut incomplete2);
            to_run
        };
        log!(
            &mut m_ctx.inner.logger,
            "... DONE FEEDING BRANCHES. {} branches to run!",
            to_run.len(),
        );
        self.poly_run_these_branches(m_ctx, protocol_description, to_run)
    }

    pub(crate) fn become_mono(
        mut self,
        decision: &Predicate,
        table_row: &mut HashMap<Key, Payload>,
    ) -> MonoP {
        if let Some((_, branch)) = self.complete.drain().find(|(p, _)| decision.satisfies(p)) {
            let BranchP { inbox, state, outbox } = branch;
            for (key, payload) in inbox.into_iter().chain(outbox.into_iter()) {
                table_row.insert(key, payload);
            }
            self.incomplete.clear();
            MonoP { state, ekeys: self.ekeys }
        } else {
            panic!("No such solution!")
        }
    }
}

impl PolyN {
    pub fn sync_recv(
        &mut self,
        ekey: Key,
        logger: &mut String,
        payload: Payload,
        payload_predicate: Predicate,
        solution_storage: &mut SolutionStorage,
    ) {
        let mut branches2: HashMap<_, _> = Default::default();
        for (old_predicate, mut branch) in self.branches.drain() {
            use CommonSatResult as Csr;
            let case = old_predicate.common_satisfier(&payload_predicate);
            let mut report_if_solution =
                |branch: &BranchN, pred: &Predicate, logger: &mut String| {
                    if branch.to_get.is_empty() {
                        solution_storage.submit_and_digest_subtree_solution(
                            logger,
                            SubtreeId::PolyN,
                            pred.clone(),
                        );
                    }
                };
            log!(
                logger,
                "Feeding msg {:?} {:?} to native branch with pred {:?}. Predicate case {:?}",
                &payload_predicate,
                &payload,
                &old_predicate,
                &case
            );
            match case {
                Csr::Nonexistant => { /* skip branch */ }
                Csr::FormerNotLatter | Csr::Equivalent => {
                    // Feed the message to this branch in-place. no need to modify pred.
                    if branch.to_get.remove(&ekey) {
                        branch.gotten.insert(ekey, payload.clone());
                        report_if_solution(&branch, &old_predicate, logger);
                    }
                }
                Csr::LatterNotFormer => {
                    // create a new branch with the payload_predicate.
                    let mut forked = branch.clone();
                    if forked.to_get.remove(&ekey) {
                        forked.gotten.insert(ekey, payload.clone());
                        report_if_solution(&forked, &payload_predicate, logger);
                        branches2.insert(payload_predicate.clone(), forked);
                    }
                }
                Csr::New(new) => {
                    // create a new branch with the newly-created predicate
                    let mut forked = branch.clone();
                    if forked.to_get.remove(&ekey) {
                        forked.gotten.insert(ekey, payload.clone());
                        report_if_solution(&forked, &new, logger);
                        branches2.insert(new.clone(), forked);
                    }
                }
            }
            // unlike PolyP machines, Native branches do not become inconsistent
            branches2.insert(old_predicate, branch);
        }
        log!(
            logger,
            "Native now has {} branches with predicates: {:?}",
            branches2.len(),
            branches2.keys().collect::<Vec<_>>()
        );
        std::mem::swap(&mut branches2, &mut self.branches);
    }

    pub fn become_mono(
        mut self,
        decision: &Predicate,
        table_row: &mut HashMap<Key, Payload>,
    ) -> MonoN {
        if let Some((_, branch)) = self
            .branches
            .drain()
            .find(|(p, branch)| branch.to_get.is_empty() && decision.satisfies(p))
        {
            let BranchN { gotten, sync_batch_index, .. } = branch;
            for (&key, payload) in gotten.iter() {
                assert!(table_row.insert(key, payload.clone()).is_none());
            }
            MonoN { ekeys: self.ekeys, result: Some((sync_batch_index, gotten)) }
        } else {
            panic!("No such solution!")
        }
    }
}
