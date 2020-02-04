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
        let cid = m_ctx.inner.channel_id_stream.controller_id;
        lockprintln!("{:?}: ~ Running branches for PolyP {:?}!", cid, m_ctx.my_subtree_id,);
        while let Some((mut predicate, mut branch)) = to_run.pop() {
            let mut r_ctx = BranchPContext {
                m_ctx: m_ctx.reborrow(),
                ekeys: &self.ekeys,
                predicate: &predicate,
                inbox: &branch.inbox,
            };
            use PolyBlocker as Sb;
            let blocker = branch.state.sync_run(&mut r_ctx, protocol_description);
            lockprintln!(
                "{:?}: ~ ... ran PolyP {:?} with branch pred {:?} to blocker {:?}",
                cid,
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
                    if predicate.replace_assignment(channel_id, true) != Some(false) {
                        // don't rerun now. Rerun at next `sync_run`
                        self.incomplete.insert(predicate, branch);
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
                    // come up with the predicate for this local solution
                    let ekeys_channel_id_iter = self
                        .ekeys
                        .iter()
                        .map(|&ekey| m_ctx.inner.endpoint_exts.get(ekey).unwrap().info.channel_id);
                    predicate.batch_assign_nones(ekeys_channel_id_iter, false);
                    // report the local solution
                    m_ctx
                        .solution_storage
                        .submit_and_digest_subtree_solution(m_ctx.my_subtree_id, predicate.clone());
                    // store the solution for recovering later
                    self.complete.insert(predicate, branch);
                }
                Sb::PutMsg(ekey, payload) => {
                    assert!(self.ekeys.contains(&ekey));
                    let EndpointExt { info, endpoint } =
                        m_ctx.inner.endpoint_exts.get_mut(ekey).unwrap();
                    if predicate.replace_assignment(info.channel_id, true) != Some(false) {
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
        let cid = m_ctx.inner.channel_id_stream.controller_id;

        let to_run = if self.complete.contains_key(&payload_predicate) {
            // exact match with stopped machine

            lockprintln!(
                "{:?}: ... poly_recv_run matched stopped machine exactly! nothing to do here",
                cid,
            );
            vec![]
        } else if let Some(mut branch) = self.incomplete.remove(&payload_predicate) {
            // exact match with running machine

            lockprintln!(
                "{:?}: ... poly_recv_run matched running machine exactly! pred is {:?}",
                cid,
                &payload_predicate
            );
            branch.inbox.insert(ekey, payload);
            vec![(payload_predicate, branch)]
        } else {
            lockprintln!(
                "{:?}: ... poly_recv_run didn't have any exact matches... Let's try feed it to all branches",
                cid,
            );
            let mut incomplete2 = HashMap::<_, _>::default();
            let to_run = self
                .incomplete
                .drain()
                .filter_map(|(old_predicate, mut branch)| {
                    use CommonSatResult as Csr;
                    match old_predicate.common_satisfier(&payload_predicate) {
                        Csr::FormerNotLatter | Csr::Equivalent => {
                            lockprintln!(
                                "{:?}: ... poly_recv_run This branch is compatible unaltered! branch pred: {:?}",
                                cid,
                                &old_predicate
                            );
                            // old_predicate COVERS the assumptions of payload_predicate
                            let was = branch.inbox.insert(ekey, payload.clone());
                            assert!(was.is_none()); // INBOX MUST BE EMPTY!
                            Some((old_predicate, branch))
                        }
                        Csr::New(new) => {

                            lockprintln!(
                                "{:?}: ... poly_recv_run payloadpred {:?} and branchpred {:?} satisfied by new pred {:?}. FORKING",
                                cid,
                                &old_predicate,
                                &payload_predicate,
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

                            lockprintln!(
                                "{:?}: ... poly_recv_run payloadpred {:?} subsumes branch pred {:?}. FORKING",
                                cid,
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
                            lockprintln!(
                                "{:?}: ... poly_recv_run SKIPPING because branchpred={:?}. payloadpred={:?}",
                                cid,
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
        lockprintln!("{:?}: ... DONE FEEDING BRANCHES. {} branches to run!", cid, to_run.len(),);
        self.poly_run_these_branches(m_ctx, protocol_description, to_run)
    }

    pub(crate) fn become_mono(
        mut self,
        decision: &Predicate,
        all_inboxes: &mut HashMap<Key, Payload>,
    ) -> MonoP {
        if let Some((_, branch)) = self.complete.drain().find(|(p, _)| decision.satisfies(p)) {
            let BranchP { inbox, state } = branch;
            for (key, payload) in inbox {
                assert!(all_inboxes.insert(key, payload).is_none());
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
        payload: Payload,
        solution_storage: &mut SolutionStorage,
    ) {
        for (predicate, branch) in self.branches.iter_mut() {
            if branch.to_get.remove(&ekey) {
                branch.gotten.insert(ekey, payload.clone());
                if branch.to_get.is_empty() {
                    solution_storage
                        .submit_and_digest_subtree_solution(SubtreeId::PolyN, predicate.clone());
                }
            }
        }
    }

    pub fn become_mono(
        mut self,
        decision: &Predicate,
        all_inboxes: &mut HashMap<Key, Payload>,
    ) -> MonoN {
        if let Some((_, branch)) = self.branches.drain().find(|(p, _)| decision.satisfies(p)) {
            let BranchN { gotten, sync_batch_index, .. } = branch;
            for (&key, payload) in gotten.iter() {
                assert!(all_inboxes.insert(key, payload.clone()).is_none());
            }
            MonoN { ekeys: self.ekeys, result: Some((sync_batch_index, gotten)) }
        } else {
            panic!("No such solution!")
        }
    }
}
