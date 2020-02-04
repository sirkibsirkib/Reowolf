use crate::common::*;
use crate::runtime::{actors::*, endpoint::*, errors::*, *};

impl Controller {
    fn end_round_with_decision(&mut self, decision: Predicate) -> Result<(), SyncErr> {
        let cid = self.inner.channel_id_stream.controller_id;
        lockprintln!("{:?}: ENDING ROUND WITH DECISION! {:?}", cid, &decision);

        let mut all_inboxes = HashMap::default();
        self.inner.mono_n = self
            .ephemeral
            .poly_n
            .take()
            .map(|poly_n| poly_n.become_mono(&decision, &mut all_inboxes));
        self.inner.mono_ps.extend(
            self.ephemeral.poly_ps.drain(..).map(|m| m.become_mono(&decision, &mut all_inboxes)),
        );
        let valuations: HashMap<_, _> = all_inboxes
            .drain()
            .map(|(ekey, payload)| {
                let channel_id = self.inner.endpoint_exts.get(ekey).unwrap().info.channel_id;
                (channel_id, Some(payload))
            })
            .collect();
        for (channel_id, value) in decision.assigned.iter() {
            if !value {
                lockprintln!("{:?}: VALUE {:?} => *", cid, channel_id);
            } else if let Some(payload) = valuations.get(channel_id) {
                lockprintln!("{:?}: VALUE {:?} => Message({:?})", cid, channel_id, payload);
            } else {
                lockprintln!("{:?}: VALUE {:?} => Message(?)", cid, channel_id);
            }
        }
        let announcement =
            CommMsgContents::Announce { oracle: decision }.into_msg(self.inner.round_index);
        for &child_ekey in self.inner.family.children_ekeys.iter() {
            lockprintln!(
                "{:?}: Forwarding {:?} to child with ekey {:?}",
                cid,
                &announcement,
                child_ekey
            );
            self.inner
                .endpoint_exts
                .get_mut(child_ekey)
                .expect("eefef")
                .endpoint
                .send(announcement.clone())?;
        }
        self.inner.round_index += 1;
        self.ephemeral.clear();
        Ok(())
    }

    // Drain self.ephemeral.solution_storage and handle the new locals. Return decision if one is found
    fn handle_locals_maybe_decide(&mut self) -> Result<bool, SyncErr> {
        let cid = self.inner.channel_id_stream.controller_id;
        if let Some(parent_ekey) = self.inner.family.parent_ekey {
            // I have a parent -> I'm not the leader
            let parent_endpoint =
                &mut self.inner.endpoint_exts.get_mut(parent_ekey).expect("huu").endpoint;
            for partial_oracle in self.ephemeral.solution_storage.iter_new_local_make_old() {
                let msg =
                    CommMsgContents::Elaborate { partial_oracle }.into_msg(self.inner.round_index);
                lockprintln!("{:?}: Sending {:?} to parent {:?}", cid, &msg, parent_ekey);
                parent_endpoint.send(msg)?;
            }
            Ok(false)
        } else {
            // I have no parent -> I'm the leader
            assert!(self.inner.family.parent_ekey.is_none());
            let maybe_decision = self.ephemeral.solution_storage.iter_new_local_make_old().next();
            Ok(if let Some(decision) = maybe_decision {
                lockprintln!("{:?}: DECIDE ON {:?} AS LEADER!", cid, &decision);
                self.end_round_with_decision(decision)?;
                true
            } else {
                false
            })
        }
    }

    fn kick_off_native(
        &mut self,
        sync_batches: impl Iterator<Item = SyncBatch>,
    ) -> Result<PolyN, EndpointErr> {
        let MonoN { ekeys, .. } = self.inner.mono_n.take().unwrap();
        let Self { inner: ControllerInner { endpoint_exts, round_index, .. }, .. } = self;
        let mut branches = HashMap::<_, _>::default();
        for (sync_batch_index, SyncBatch { puts, gets }) in sync_batches.enumerate() {
            let ekey_to_channel_id = |ekey| endpoint_exts.get(ekey).unwrap().info.channel_id;
            let all_ekeys = ekeys.iter().copied();
            let all_channel_ids = all_ekeys.map(ekey_to_channel_id);

            let mut predicate = Predicate::new_trivial();

            // assign TRUE for puts and gets
            let true_ekeys = puts.keys().chain(gets.iter()).copied();
            let true_channel_ids = true_ekeys.clone().map(ekey_to_channel_id);
            predicate.batch_assign_nones(true_channel_ids, true);

            // assign FALSE for all in interface not assigned true
            predicate.batch_assign_nones(all_channel_ids.clone(), false);

            if branches.contains_key(&predicate) {
                // TODO what do I do with redundant predicates?
                unimplemented!(
                    "Having multiple batches with the same
                    predicate requires the support of oracle boolean variables"
                )
            }
            let branch = BranchN {
                to_get: true_ekeys.collect(),
                gotten: Default::default(),
                sync_batch_index,
            };
            for (ekey, payload) in puts {
                let msg =
                    CommMsgContents::SendPayload { payload_predicate: predicate.clone(), payload }
                        .into_msg(*round_index);
                endpoint_exts.get_mut(ekey).unwrap().endpoint.send(msg)?;
            }
            if branch.to_get.is_empty() {
                self.ephemeral
                    .solution_storage
                    .submit_and_digest_subtree_solution(SubtreeId::PolyN, predicate.clone());
            }
            branches.insert(predicate, branch);
        }
        Ok(PolyN { ekeys, branches })
    }

    // Runs a synchronous round until all the actors are in decided state OR 1+ are inconsistent.
    // If a native requires setting up, arg `sync_batches` is Some, and those are used as the sync batches.
    pub fn sync_round(
        &mut self,
        deadline: Instant,
        sync_batches: Option<impl Iterator<Item = SyncBatch>>,
    ) -> Result<(), SyncErr> {
        // TODO! fuse handle_locals_return_decision and end_round_return_decision

        assert!(self.ephemeral.is_clear());

        let cid = self.inner.channel_id_stream.controller_id;
        lockprintln!();
        lockprintln!("~~~~~~ {:?}: SYNC ROUND STARTS! ROUND={}", cid, self.inner.round_index);

        // 1. Run the Mono for each Mono actor (stored in `self.mono_ps`).
        //    Some actors are dropped. some new actors are created.
        //    Ultimately, we have 0 Mono actors and a list of unnamed sync_actors
        lockprintln!("{:?}: Got {} MonoP's to run!", cid, self.inner.mono_ps.len());
        self.ephemeral.poly_ps.clear();
        // let mut poly_ps: Vec<PolyP> = vec![];
        while let Some(mut mono_p) = self.inner.mono_ps.pop() {
            let mut m_ctx = MonoPContext {
                ekeys: &mut mono_p.ekeys,
                inner: &mut self.inner,
                // endpoint_exts: &mut self.endpoint_exts,
                // mono_ps: &mut self.mono_ps,
                // channel_id_stream: &mut self.channel_id_stream,
            };
            // cross boundary into crate::protocol
            let blocker = mono_p.state.pre_sync_run(&mut m_ctx, &self.protocol_description);
            lockprintln!("{:?}: ... MonoP's pre_sync_run got blocker {:?}", cid, &blocker);
            match blocker {
                MonoBlocker::Inconsistent => return Err(SyncErr::Inconsistent),
                MonoBlocker::ComponentExit => drop(mono_p),
                MonoBlocker::SyncBlockStart => self.ephemeral.poly_ps.push(mono_p.into()),
            }
        }
        lockprintln!(
            "{:?}: Finished running all MonoPs! Have {} PolyPs waiting",
            cid,
            self.ephemeral.poly_ps.len()
        );

        // 3. define the mapping from ekey -> actor
        //    this is needed during the event loop to determine which actor
        //    should receive the incoming message.
        //    TODO: store and update this mapping rather than rebuilding it each round.
        let ekey_to_holder: HashMap<Key, PolyId> = {
            use PolyId::*;
            let n = self.inner.mono_n.iter().flat_map(|m| m.ekeys.iter().map(move |&e| (e, N)));
            let p = self
                .ephemeral
                .poly_ps
                .iter()
                .enumerate()
                .flat_map(|(index, m)| m.ekeys.iter().map(move |&e| (e, P { index })));
            n.chain(p).collect()
        };
        lockprintln!(
            "{:?}: SET OF PolyPs and MonoPs final! ekey lookup map is {:?}",
            cid,
            &ekey_to_holder
        );

        // 4. Create the solution storage. it tracks the solutions of "subtrees"
        //    of the controller in the overlay tree.
        self.ephemeral.solution_storage.reset({
            let n = self.inner.mono_n.iter().map(|_| SubtreeId::PolyN);
            let m = (0..self.ephemeral.poly_ps.len()).map(|index| SubtreeId::PolyP { index });
            let c = self
                .inner
                .family
                .children_ekeys
                .iter()
                .map(|&ekey| SubtreeId::ChildController { ekey });
            let subtree_id_iter = n.chain(m).chain(c);
            lockprintln!(
                "{:?}: Solution Storage has subtree Ids: {:?}",
                cid,
                &subtree_id_iter.clone().collect::<Vec<_>>()
            );
            subtree_id_iter
        });

        // 5. kick off the synchronous round of the native actor if it exists

        lockprintln!("{:?}: Kicking off native's synchronous round...", cid);
        assert_eq!(sync_batches.is_some(), self.inner.mono_n.is_some()); // TODO better err
        self.ephemeral.poly_n = if let Some(sync_batches) = sync_batches {
            // using if let because of nested ? operator
            // TODO check that there are 1+ branches or NO SOLUTION
            let poly_n = self.kick_off_native(sync_batches)?;
            lockprintln!(
                "{:?}: PolyN kicked off, and has branches with predicates... {:?}",
                cid,
                poly_n.branches.keys().collect::<Vec<_>>()
            );
            Some(poly_n)
        } else {
            lockprintln!("{:?}: NO NATIVE COMPONENT", cid);
            None
        };

        // 6. Kick off the synchronous round of each protocol actor
        //    If just one actor becomes inconsistent now, there can be no solution!
        //    TODO distinguish between completed and not completed poly_p's?
        lockprintln!("{:?}: Kicking off {} PolyP's.", cid, self.ephemeral.poly_ps.len());
        for (index, poly_p) in self.ephemeral.poly_ps.iter_mut().enumerate() {
            let my_subtree_id = SubtreeId::PolyP { index };
            let m_ctx = PolyPContext {
                my_subtree_id,
                inner: &mut self.inner,
                solution_storage: &mut self.ephemeral.solution_storage,
            };
            use SyncRunResult as Srr;
            let blocker = poly_p.poly_run(m_ctx, &self.protocol_description)?;
            lockprintln!("{:?}: ... PolyP's poly_run got blocker {:?}", cid, &blocker);
            match blocker {
                Srr::NoBranches => return Err(SyncErr::Inconsistent),
                Srr::AllBranchesComplete | Srr::BlockingForRecv => (),
            }
        }
        lockprintln!("{:?}: All Poly machines have been kicked off!", cid);

        // 7. `solution_storage` may have new solutions for this controller
        //    handle their discovery. LEADER => announce, otherwise => send to parent
        {
            let peeked = self.ephemeral.solution_storage.peek_new_locals().collect::<Vec<_>>();
            lockprintln!(
                "{:?}: Got {} controller-local solutions before a single RECV: {:?}",
                cid,
                peeked.len(),
                peeked
            );
        }
        if self.handle_locals_maybe_decide()? {
            return Ok(());
        }

        // 4. Receive incoming messages until the DECISION is made
        lockprintln!("{:?}: No decision yet. Time to recv messages", cid);
        self.undelay_all();
        'recv_loop: loop {
            let received = self.recv(deadline)?.ok_or(SyncErr::Timeout)?;
            let current_content = match received.msg {
                Msg::SetupMsg(_) => {
                    lockprintln!("{:?}: recvd message {:?} and its SETUP :(", cid, &received);
                    // This occurs in the event the connector was malformed during connect()
                    return Err(SyncErr::UnexpectedSetupMsg);
                }
                Msg::CommMsg(CommMsg { round_index, .. })
                    if round_index < self.inner.round_index =>
                {
                    // Old message! Can safely discard
                    lockprintln!("{:?}: recvd message {:?} and its OLD! :(", cid, &received);
                    drop(received);
                    continue 'recv_loop;
                }
                Msg::CommMsg(CommMsg { round_index, .. })
                    if round_index > self.inner.round_index =>
                {
                    // Message from a next round. Keep for later!
                    lockprintln!(
                        "{:?}: recvd message {:?} and its for later. DELAY! :(",
                        cid,
                        &received
                    );
                    self.delay(received);
                    continue 'recv_loop;
                }
                Msg::CommMsg(CommMsg { contents, round_index }) => {
                    lockprintln!("{:?}: recvd a round-appropriate CommMsg {:?}", cid, &contents);
                    assert_eq!(round_index, self.inner.round_index);
                    contents
                }
            };
            match current_content {
                CommMsgContents::Elaborate { partial_oracle } => {
                    // Child controller submitted a subtree solution.
                    if !self.inner.family.children_ekeys.contains(&received.recipient) {
                        return Err(SyncErr::ElaborateFromNonChild);
                    }
                    let subtree_id = SubtreeId::ChildController { ekey: received.recipient };
                    lockprintln!(
                        "{:?}: Received elaboration from child for subtree {:?}: {:?}",
                        cid,
                        subtree_id,
                        &partial_oracle
                    );
                    self.ephemeral
                        .solution_storage
                        .submit_and_digest_subtree_solution(subtree_id, partial_oracle);

                    if self.handle_locals_maybe_decide()? {
                        return Ok(());
                    }
                }
                CommMsgContents::Announce { oracle } => {
                    if self.inner.family.parent_ekey != Some(received.recipient) {
                        return Err(SyncErr::AnnounceFromNonParent);
                    }
                    lockprintln!(
                        "{:?}: Received ANNOUNCEMENT from from parent {:?}: {:?}",
                        cid,
                        received.recipient,
                        &oracle
                    );
                    return self.end_round_with_decision(oracle);
                }
                CommMsgContents::SendPayload { payload_predicate, payload } => {
                    // message for some actor. Feed it to the appropriate actor
                    // and then give them another chance to run.
                    let subtree_id = ekey_to_holder.get(&received.recipient);
                    lockprintln!(
                        "{:?}: Received SendPayload for subtree {:?} with pred {:?} and payload {:?}",
                        cid, subtree_id, &payload_predicate, &payload
                    );
                    match subtree_id {
                        None => {
                            // this happens when a message is sent to a component that has exited.
                            // It's safe to drop this message;
                            // The sender branch will certainly not be part of the solution
                            continue 'recv_loop;
                        }
                        Some(PolyId::N) => {
                            // Message for NativeMachine
                            self.ephemeral.poly_n.as_mut().unwrap().sync_recv(
                                received.recipient,
                                payload,
                                &mut self.ephemeral.solution_storage,
                            );
                        }
                        Some(PolyId::P { index }) => {
                            // Message for protocol actor
                            let channel_id = self
                                .inner
                                .endpoint_exts
                                .get(received.recipient)
                                .expect("UEHFU")
                                .info
                                .channel_id;
                            if payload_predicate.query(channel_id) != Some(true) {
                                // sender didn't preserve the invariant
                                return Err(SyncErr::PayloadPremiseExcludesTheChannel(channel_id));
                            }
                            let poly_p = &mut self.ephemeral.poly_ps[*index];

                            let m_ctx = PolyPContext {
                                my_subtree_id: SubtreeId::PolyP { index: *index },
                                inner: &mut self.inner,
                                solution_storage: &mut self.ephemeral.solution_storage,
                            };
                            use SyncRunResult as Srr;
                            let blocker = poly_p.poly_recv_run(
                                m_ctx,
                                &self.protocol_description,
                                received.recipient,
                                payload_predicate,
                                payload,
                            )?;
                            lockprintln!(
                                "{:?}: ... Fed the msg to PolyP {:?} and ran it to blocker {:?}",
                                cid,
                                subtree_id,
                                blocker
                            );
                            match blocker {
                                Srr::NoBranches => return Err(SyncErr::Inconsistent),
                                Srr::BlockingForRecv | Srr::AllBranchesComplete => {
                                    continue 'recv_loop
                                }
                            }
                        }
                    };
                    {
                        let peeked =
                            self.ephemeral.solution_storage.peek_new_locals().collect::<Vec<_>>();
                        lockprintln!(
                            "{:?}: Got {} new controller-local solutions from RECV: {:?}",
                            cid,
                            peeked.len(),
                            peeked
                        );
                    }
                    if self.handle_locals_maybe_decide()? {
                        return Ok(());
                    }
                }
            }
        }
    }
}
impl ControllerEphemeral {
    fn is_clear(&self) -> bool {
        self.solution_storage.is_clear()
            && self.poly_n.is_none()
            && self.poly_ps.is_empty()
            && self.ekey_to_holder.is_empty()
    }
    fn clear(&mut self) {
        self.solution_storage.clear();
        self.poly_n.take();
        self.poly_ps.clear();
        self.ekey_to_holder.clear();
    }
}
impl Into<PolyP> for MonoP {
    fn into(self) -> PolyP {
        PolyP {
            complete: Default::default(),
            incomplete: hashmap! {
                Predicate::new_trivial() =>
                BranchP {
                    state: self.state,
                    inbox: Default::default(),
                }
            },
            ekeys: self.ekeys,
        }
    }
}

impl From<EndpointErr> for SyncErr {
    fn from(e: EndpointErr) -> SyncErr {
        SyncErr::EndpointErr(e)
    }
}

impl MonoContext for MonoPContext<'_> {
    type D = ProtocolD;
    type S = ProtocolS;
    fn new_component(&mut self, moved_ekeys: HashSet<Key>, init_state: Self::S) {
        lockprintln!(
            "{:?}: !! MonoContext callback to new_component with ekeys {:?}!",
            self.inner.channel_id_stream.controller_id,
            &moved_ekeys,
        );
        if moved_ekeys.is_subset(self.ekeys) {
            self.ekeys.retain(|x| !moved_ekeys.contains(x));
            self.inner.mono_ps.push(MonoP { state: init_state, ekeys: moved_ekeys });
        } else {
            panic!("MachineP attempting to move alien ekey!");
        }
    }
    fn new_channel(&mut self) -> [Key; 2] {
        let [a, b] = Endpoint::new_memory_pair();
        let channel_id = self.inner.channel_id_stream.next();
        let kp = self.inner.endpoint_exts.alloc(EndpointExt {
            info: EndpointInfo { polarity: Putter, channel_id },
            endpoint: a,
        });
        let kg = self.inner.endpoint_exts.alloc(EndpointExt {
            info: EndpointInfo { polarity: Putter, channel_id },
            endpoint: b,
        });
        lockprintln!(
            "{:?}: !! MonoContext callback to new_channel. returning ekeys {:?}!",
            self.inner.channel_id_stream.controller_id,
            [kp, kg],
        );
        [kp, kg]
    }
    fn new_random(&self) -> u64 {
        type Bytes8 = [u8; std::mem::size_of::<u64>()];
        let mut bytes = Bytes8::default();
        getrandom::getrandom(&mut bytes).unwrap();
        let val = unsafe { std::mem::transmute::<Bytes8, _>(bytes) };
        lockprintln!(
            "{:?}: !! MonoContext callback to new_random. returning val {:?}!",
            self.inner.channel_id_stream.controller_id,
            val,
        );
        val
    }
}

impl SolutionStorage {
    fn is_clear(&self) -> bool {
        self.subtree_id_to_index.is_empty()
            && self.subtree_solutions.is_empty()
            && self.old_local.is_empty()
            && self.new_local.is_empty()
    }
    fn clear(&mut self) {
        self.subtree_id_to_index.clear();
        self.subtree_solutions.clear();
        self.old_local.clear();
        self.new_local.clear();
    }
    pub(crate) fn reset(&mut self, subtree_ids: impl Iterator<Item = SubtreeId>) {
        self.subtree_id_to_index.clear();
        self.subtree_solutions.clear();
        self.old_local.clear();
        self.new_local.clear();
        for key in subtree_ids {
            self.subtree_id_to_index.insert(key, self.subtree_solutions.len());
            self.subtree_solutions.push(Default::default())
        }
    }

    pub(crate) fn peek_new_locals(&self) -> impl Iterator<Item = &Predicate> + '_ {
        self.new_local.iter()
    }

    pub(crate) fn iter_new_local_make_old(&mut self) -> impl Iterator<Item = Predicate> + '_ {
        let Self { old_local, new_local, .. } = self;
        new_local.drain().map(move |local| {
            old_local.insert(local.clone());
            local
        })
    }

    pub(crate) fn submit_and_digest_subtree_solution(
        &mut self,
        subtree_id: SubtreeId,
        predicate: Predicate,
    ) {
        let index = self.subtree_id_to_index[&subtree_id];
        let left = 0..index;
        let right = (index + 1)..self.subtree_solutions.len();

        let Self { subtree_solutions, new_local, old_local, .. } = self;
        let was_new = subtree_solutions[index].insert(predicate.clone());
        if was_new {
            let set_visitor = left.chain(right).map(|index| &subtree_solutions[index]);
            Self::elaborate_into_new_local_rec(predicate, set_visitor, old_local, new_local);
        }
    }

    fn elaborate_into_new_local_rec<'a, 'b>(
        partial: Predicate,
        mut set_visitor: impl Iterator<Item = &'b HashSet<Predicate>> + Clone,
        old_local: &'b HashSet<Predicate>,
        new_local: &'a mut HashSet<Predicate>,
    ) {
        if let Some(set) = set_visitor.next() {
            // incomplete solution. keep traversing
            for pred in set.iter() {
                if let Some(elaborated) = pred.union_with(&partial) {
                    Self::elaborate_into_new_local_rec(
                        elaborated,
                        set_visitor.clone(),
                        old_local,
                        new_local,
                    )
                }
            }
        } else {
            // recursive stop condition. `partial` is a local subtree solution
            if !old_local.contains(&partial) {
                // ... and it hasn't been found before
                new_local.insert(partial);
            }
        }
    }
}
impl PolyContext for BranchPContext<'_, '_> {
    type D = ProtocolD;

    fn is_firing(&self, ekey: Key) -> Option<bool> {
        assert!(self.ekeys.contains(&ekey));
        let channel_id = self.m_ctx.inner.endpoint_exts.get(ekey).unwrap().info.channel_id;
        let val = self.predicate.query(channel_id);
        lockprintln!(
            "{:?}: !! PolyContext callback to is_firing by {:?}! returning {:?}",
            self.m_ctx.inner.channel_id_stream.controller_id,
            self.m_ctx.my_subtree_id,
            val,
        );
        val
    }
    fn read_msg(&self, ekey: Key) -> Option<&Payload> {
        assert!(self.ekeys.contains(&ekey));
        let val = self.inbox.get(&ekey);
        lockprintln!(
            "{:?}: !! PolyContext callback to read_msg by {:?}! returning {:?}",
            self.m_ctx.inner.channel_id_stream.controller_id,
            self.m_ctx.my_subtree_id,
            val,
        );
        val
    }
}
