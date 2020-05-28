use crate::common::*;
use crate::runtime::{actors::*, endpoint::*, errors::*, *};

impl Controller {
    fn end_round_with_decision(&mut self, decision: Decision) -> Result<(), SyncErr> {
        log!(&mut self.inner.logger, "ENDING ROUND WITH DECISION! {:?}", &decision);
        let ret = match &decision {
            Decision::Success(predicate) => {
                // overwrite MonoN/P
                self.inner.mono_n = {
                    let poly_n = self.ephemeral.poly_n.take().unwrap();
                    poly_n.choose_mono(predicate).unwrap_or_else(|| {
                        panic!(
                            "Ending round with decision pred {:#?} but poly_n has branches {:#?}. My log is... {}",
                            &predicate, &poly_n.branches, &self.inner.logger
                        );
                    })
                };
                self.inner.mono_ps.clear();
                self.inner.mono_ps.extend(
                    self.ephemeral
                        .poly_ps
                        .drain(..)
                        .map(|poly_p| poly_p.choose_mono(predicate).unwrap()),
                );
                Ok(())
            }
            Decision::Failure => Err(SyncErr::Timeout),
        };
        let announcement = CommMsgContents::Announce { decision }.into_msg(self.inner.round_index);
        for &child_port in self.inner.family.children_ports.iter() {
            log!(
                &mut self.inner.logger,
                "Forwarding {:?} to child with port {:?}",
                &announcement,
                child_port
            );
            self.inner
                .endpoint_exts
                .get_mut(child_port)
                .expect("eefef")
                .endpoint
                .send(announcement.clone())?;
        }
        self.inner.round_index += 1;
        self.ephemeral.clear();
        ret
    }

    // Drain self.ephemeral.solution_storage and handle the new locals. Return decision if one is found
    fn handle_locals_maybe_decide(&mut self) -> Result<bool, SyncErr> {
        if let Some(parent_port) = self.inner.family.parent_port {
            // I have a parent -> I'm not the leader
            let parent_endpoint =
                &mut self.inner.endpoint_exts.get_mut(parent_port).expect("huu").endpoint;
            for partial_oracle in self.ephemeral.solution_storage.iter_new_local_make_old() {
                let msg =
                    CommMsgContents::Elaborate { partial_oracle }.into_msg(self.inner.round_index);
                log!(&mut self.inner.logger, "Sending {:?} to parent {:?}", &msg, parent_port);
                parent_endpoint.send(msg)?;
            }
            Ok(false)
        } else {
            // I have no parent -> I'm the leader
            assert!(self.inner.family.parent_port.is_none());
            let maybe_predicate = self.ephemeral.solution_storage.iter_new_local_make_old().next();
            Ok(if let Some(predicate) = maybe_predicate {
                let decision = Decision::Success(predicate);
                log!(&mut self.inner.logger, "DECIDE ON {:?} AS LEADER!", &decision);
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
        let MonoN { ports, .. } = self.inner.mono_n.clone();
        let Self { inner: ControllerInner { endpoint_exts, round_index, .. }, .. } = self;
        let mut branches = HashMap::<_, _>::default();
        for (sync_batch_index, SyncBatch { puts, gets }) in sync_batches.enumerate() {
            let port_to_channel_id = |port| endpoint_exts.get(port).unwrap().info.channel_id;
            let all_ports = ports.iter().copied();
            let all_channel_ids = all_ports.map(port_to_channel_id);

            let mut predicate = Predicate::new_trivial();

            // assign TRUE for puts and gets
            let true_ports = puts.keys().chain(gets.iter()).copied();
            let true_channel_ids = true_ports.clone().map(port_to_channel_id);
            predicate.batch_assign_nones(true_channel_ids, true);

            // assign FALSE for all in interface not assigned true
            predicate.batch_assign_nones(all_channel_ids.clone(), false);

            if branches.contains_key(&predicate) {
                // TODO what do I do with redundant predicates?
                unimplemented!(
                    "Duplicate predicate {:#?}!\nHaving multiple batches with the same
                    predicate requires the support of oracle boolean variables",
                    &predicate,
                )
            }
            let branch = BranchN { to_get: gets, gotten: Default::default(), sync_batch_index };
            for (port, payload) in puts {
                log!(
                    &mut self.inner.logger,
                    "... ... Initial native put msg {:?} pred {:?} batch {:?}",
                    &payload,
                    &predicate,
                    sync_batch_index,
                );
                let msg =
                    CommMsgContents::SendPayload { payload_predicate: predicate.clone(), payload }
                        .into_msg(*round_index);
                endpoint_exts.get_mut(port).unwrap().endpoint.send(msg)?;
            }
            log!(
                &mut self.inner.logger,
                "... Initial native branch batch index={} with pred {:?}",
                sync_batch_index,
                &predicate
            );
            if branch.to_get.is_empty() {
                self.ephemeral.solution_storage.submit_and_digest_subtree_solution(
                    &mut self.inner.logger,
                    SubtreeId::PolyN,
                    predicate.clone(),
                );
            }
            branches.insert(predicate, branch);
        }
        Ok(PolyN { ports, branches })
    }
    pub fn sync_round(
        &mut self,
        deadline: Option<Instant>,
        sync_batches: Option<impl Iterator<Item = SyncBatch>>,
    ) -> Result<(), SyncErr> {
        if let Some(e) = self.unrecoverable_error {
            return Err(e.clone());
        }
        self.sync_round_inner(deadline, sync_batches).map_err(move |e| match e {
            SyncErr::Timeout => e, // this isn't unrecoverable
            _ => {
                // Must set unrecoverable error! and tear down our net channels
                self.unrecoverable_error = Some(e);
                self.ephemeral.clear();
                self.inner.endpoint_exts = Default::default();
                e
            }
        })
    }

    // Runs a synchronous round until all the actors are in decided state OR 1+ are inconsistent.
    // If a native requires setting up, arg `sync_batches` is Some, and those are used as the sync batches.
    fn sync_round_inner(
        &mut self,
        mut deadline: Option<Instant>,
        sync_batches: Option<impl Iterator<Item = SyncBatch>>,
    ) -> Result<(), SyncErr> {
        log!(
            &mut self.inner.logger,
            "~~~~~~~~ SYNC ROUND STARTS! ROUND={} ~~~~~~~~~",
            self.inner.round_index
        );
        assert!(self.ephemeral.is_clear());
        assert!(self.unrecoverable_error.is_none());

        // 1. Run the Mono for each Mono actor (stored in `self.mono_ps`).
        //    Some actors are dropped. some new actors are created.
        //    Ultimately, we have 0 Mono actors and a list of unnamed sync_actors
        self.ephemeral.mono_ps.extend(self.inner.mono_ps.iter().cloned());
        log!(&mut self.inner.logger, "Got {} MonoP's to run!", self.ephemeral.mono_ps.len());
        while let Some(mut mono_p) = self.ephemeral.mono_ps.pop() {
            let mut m_ctx = MonoPContext {
                ports: &mut mono_p.ports,
                mono_ps: &mut self.ephemeral.mono_ps,
                inner: &mut self.inner,
            };
            // cross boundary into crate::protocol
            let blocker = mono_p.state.pre_sync_run(&mut m_ctx, &self.protocol_description);
            log!(&mut self.inner.logger, "... MonoP's pre_sync_run got blocker {:?}", &blocker);
            match blocker {
                MonoBlocker::Inconsistent => return Err(SyncErr::Inconsistent),
                MonoBlocker::ComponentExit => drop(mono_p),
                MonoBlocker::SyncBlockStart => self.ephemeral.poly_ps.push(mono_p.into()),
            }
        }
        log!(
            &mut self.inner.logger,
            "Finished running all MonoPs! Have {} PolyPs waiting",
            self.ephemeral.poly_ps.len()
        );

        // 3. define the mapping from port -> actor
        //    this is needed during the event loop to determine which actor
        //    should receive the incoming message.
        //    TODO: store and update this mapping rather than rebuilding it each round.
        let port_to_holder: HashMap<Port, PolyId> = {
            use PolyId::*;
            let n = self.inner.mono_n.ports.iter().map(move |&e| (e, N));
            let p = self
                .ephemeral
                .poly_ps
                .iter()
                .enumerate()
                .flat_map(|(index, m)| m.ports.iter().map(move |&e| (e, P { index })));
            n.chain(p).collect()
        };
        log!(
            &mut self.inner.logger,
            "SET OF PolyPs and MonoPs final! port lookup map is {:?}",
            &port_to_holder
        );

        // 4. Create the solution storage. it tracks the solutions of "subtrees"
        //    of the controller in the overlay tree.
        self.ephemeral.solution_storage.reset({
            let n = std::iter::once(SubtreeId::PolyN);
            let m = (0..self.ephemeral.poly_ps.len()).map(|index| SubtreeId::PolyP { index });
            let c = self
                .inner
                .family
                .children_ports
                .iter()
                .map(|&port| SubtreeId::ChildController { port });
            let subtree_id_iter = n.chain(m).chain(c);
            log!(
                &mut self.inner.logger,
                "Solution Storage has subtree Ids: {:?}",
                &subtree_id_iter.clone().collect::<Vec<_>>()
            );
            subtree_id_iter
        });

        // 5. kick off the synchronous round of the native actor if it exists

        log!(&mut self.inner.logger, "Kicking off native's synchronous round...");
        self.ephemeral.poly_n = if let Some(sync_batches) = sync_batches {
            // using if let because of nested ? operator
            // TODO check that there are 1+ branches or NO SOLUTION
            let poly_n = self.kick_off_native(sync_batches)?;
            log!(
                &mut self.inner.logger,
                "PolyN kicked off, and has branches with predicates... {:?}",
                poly_n.branches.keys().collect::<Vec<_>>()
            );
            Some(poly_n)
        } else {
            log!(&mut self.inner.logger, "NO NATIVE COMPONENT");
            None
        };

        // 6. Kick off the synchronous round of each protocol actor
        //    If just one actor becomes inconsistent now, there can be no solution!
        //    TODO distinguish between completed and not completed poly_p's?
        log!(&mut self.inner.logger, "Kicking off {} PolyP's.", self.ephemeral.poly_ps.len());
        for (index, poly_p) in self.ephemeral.poly_ps.iter_mut().enumerate() {
            let my_subtree_id = SubtreeId::PolyP { index };
            let m_ctx = PolyPContext {
                my_subtree_id,
                inner: &mut self.inner,
                solution_storage: &mut self.ephemeral.solution_storage,
            };
            use SyncRunResult as Srr;
            let blocker = poly_p.poly_run(m_ctx, &self.protocol_description)?;
            log!(&mut self.inner.logger, "... PolyP's poly_run got blocker {:?}", &blocker);
            match blocker {
                Srr::NoBranches => return Err(SyncErr::Inconsistent),
                Srr::AllBranchesComplete | Srr::BlockingForRecv => (),
            }
        }
        log!(&mut self.inner.logger, "All Poly machines have been kicked off!");

        // 7. `solution_storage` may have new solutions for this controller
        //    handle their discovery. LEADER => announce, otherwise => send to parent
        {
            let peeked = self.ephemeral.solution_storage.peek_new_locals().collect::<Vec<_>>();
            log!(
                &mut self.inner.logger,
                "Got {} controller-local solutions before a single RECV: {:?}",
                peeked.len(),
                peeked
            );
        }
        if self.handle_locals_maybe_decide()? {
            return Ok(());
        }

        // 4. Receive incoming messages until the DECISION is made OR some unrecoverable error
        log!(&mut self.inner.logger, "`No decision yet`. Time to recv messages");
        self.undelay_all();
        'recv_loop: loop {
            log!(&mut self.inner.logger, "`POLLING` with deadline {:?}...", deadline);
            let received = match deadline {
                None => {
                    // we have personally timed out. perform a "long" poll.
                    self.recv(Instant::now() + Duration::from_secs(10))?.expect("DRIED UP")
                }
                Some(d) => match self.recv(d)? {
                    // we have not yet timed out. performed a time-limited poll
                    Some(received) => received,
                    None => {
                        // timed out! send a FAILURE message to the sink,
                        // and henceforth don't time out on polling.
                        deadline = None;
                        match self.inner.family.parent_port {
                            None => {
                                // I am the sink! announce failure and return.
                                return self.end_round_with_decision(Decision::Failure);
                            }
                            Some(parent_port) => {
                                // I am not the sink! send a failure message.
                                let announcement = Msg::CommMsg(CommMsg {
                                    round_index: self.inner.round_index,
                                    contents: CommMsgContents::Failure,
                                });
                                log!(
                                    &mut self.inner.logger,
                                    "Forwarding {:?} to parent with port {:?}",
                                    &announcement,
                                    parent_port
                                );
                                self.inner
                                    .endpoint_exts
                                    .get_mut(parent_port)
                                    .expect("ss")
                                    .endpoint
                                    .send(announcement.clone())?;
                                continue; // poll some more
                            }
                        }
                    }
                },
            };
            log!(&mut self.inner.logger, "::: message {:?}...", &received);
            let current_content = match received.msg {
                Msg::SetupMsg(s) => {
                    // This occurs in the event the connector was malformed during connect()
                    println!("WASNT EXPECTING {:?}", s);
                    return Err(SyncErr::UnexpectedSetupMsg);
                }
                Msg::CommMsg(CommMsg { round_index, .. })
                    if round_index < self.inner.round_index =>
                {
                    // Old message! Can safely discard
                    log!(&mut self.inner.logger, "...and its OLD! :(");
                    drop(received);
                    continue 'recv_loop;
                }
                Msg::CommMsg(CommMsg { round_index, .. })
                    if round_index > self.inner.round_index =>
                {
                    // Message from a next round. Keep for later!
                    log!(&mut self.inner.logger, "... DELAY! :(");
                    self.delay(received);
                    continue 'recv_loop;
                }
                Msg::CommMsg(CommMsg { contents, round_index }) => {
                    log!(
                        &mut self.inner.logger,
                        "... its a round-appropriate CommMsg with port {:?}",
                        received.recipient
                    );
                    assert_eq!(round_index, self.inner.round_index);
                    contents
                }
            };
            match current_content {
                CommMsgContents::Failure => match self.inner.family.parent_port {
                    Some(parent_port) => {
                        let announcement = Msg::CommMsg(CommMsg {
                            round_index: self.inner.round_index,
                            contents: CommMsgContents::Failure,
                        });
                        log!(
                            &mut self.inner.logger,
                            "Forwarding {:?} to parent with port {:?}",
                            &announcement,
                            parent_port
                        );
                        self.inner
                            .endpoint_exts
                            .get_mut(parent_port)
                            .expect("ss")
                            .endpoint
                            .send(announcement.clone())?;
                    }
                    None => return self.end_round_with_decision(Decision::Failure),
                },
                CommMsgContents::Elaborate { partial_oracle } => {
                    // Child controller submitted a subtree solution.
                    if !self.inner.family.children_ports.contains(&received.recipient) {
                        return Err(SyncErr::ElaborateFromNonChild);
                    }
                    let subtree_id = SubtreeId::ChildController { port: received.recipient };
                    log!(
                        &mut self.inner.logger,
                        "Received elaboration from child for subtree {:?}: {:?}",
                        subtree_id,
                        &partial_oracle
                    );
                    self.ephemeral.solution_storage.submit_and_digest_subtree_solution(
                        &mut self.inner.logger,
                        subtree_id,
                        partial_oracle,
                    );
                    if self.handle_locals_maybe_decide()? {
                        return Ok(());
                    }
                }
                CommMsgContents::Announce { decision } => {
                    if self.inner.family.parent_port != Some(received.recipient) {
                        return Err(SyncErr::AnnounceFromNonParent);
                    }
                    log!(
                        &mut self.inner.logger,
                        "Received ANNOUNCEMENT from from parent {:?}: {:?}",
                        received.recipient,
                        &decision
                    );
                    return self.end_round_with_decision(decision);
                }
                CommMsgContents::SendPayload { payload_predicate, payload } => {
                    // check that we expect to be able to receive payloads from this sender
                    assert_eq!(
                        Getter,
                        self.inner.endpoint_exts.get(received.recipient).unwrap().info.polarity
                    );

                    // message for some actor. Feed it to the appropriate actor
                    // and then give them another chance to run.
                    let subtree_id = port_to_holder.get(&received.recipient);
                    log!(
                        &mut self.inner.logger,
                        "Received SendPayload for subtree {:?} with pred {:?} and payload {:?}",
                        subtree_id,
                        &payload_predicate,
                        &payload
                    );
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
                    match subtree_id {
                        None => {
                            // this happens when a message is sent to a component that has exited.
                            // It's safe to drop this message;
                            // The sender branch will certainly not be part of the solution
                        }
                        Some(PolyId::N) => {
                            // Message for NativeMachine
                            self.ephemeral.poly_n.as_mut().unwrap().sync_recv(
                                received.recipient,
                                &mut self.inner.logger,
                                payload,
                                payload_predicate,
                                &mut self.ephemeral.solution_storage,
                            );
                            if self.handle_locals_maybe_decide()? {
                                return Ok(());
                            }
                        }
                        Some(PolyId::P { index }) => {
                            // Message for protocol actor
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
                            log!(
                                &mut self.inner.logger,
                                "... Fed the msg to PolyP {:?} and ran it to blocker {:?}",
                                subtree_id,
                                blocker
                            );
                            match blocker {
                                Srr::NoBranches => return Err(SyncErr::Inconsistent),
                                Srr::BlockingForRecv | Srr::AllBranchesComplete => {
                                    {
                                        let peeked = self
                                            .ephemeral
                                            .solution_storage
                                            .peek_new_locals()
                                            .collect::<Vec<_>>();
                                        log!(
                                            &mut self.inner.logger,
                                            "Got {} new controller-local solutions from RECV: {:?}",
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
                    };
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
            && self.mono_ps.is_empty()
            && self.port_to_holder.is_empty()
    }
    fn clear(&mut self) {
        self.solution_storage.clear();
        self.poly_n.take();
        self.poly_ps.clear();
        self.port_to_holder.clear();
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
                    outbox: Default::default(),
                    blocking_on: None,
                }
            },
            ports: self.ports,
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
    fn new_component(&mut self, moved_ports: HashSet<Port>, init_state: Self::S) {
        log!(
            &mut self.inner.logger,
            "!! MonoContext callback to new_component with ports {:?}!",
            &moved_ports,
        );
        if moved_ports.is_subset(self.ports) {
            self.ports.retain(|x| !moved_ports.contains(x));
            self.mono_ps.push(MonoP { state: init_state, ports: moved_ports });
        } else {
            panic!("MachineP attempting to move alien port!");
        }
    }
    fn new_channel(&mut self) -> [Port; 2] {
        let [a, b] = Endpoint::new_memory_pair();
        let channel_id = self.inner.channel_id_stream.next();

        let mut clos = |endpoint, polarity| {
            let endpoint_ext =
                EndpointExt { info: EndpointInfo { polarity, channel_id }, endpoint };
            let port = self.inner.endpoint_exts.alloc(endpoint_ext);
            let endpoint = &self.inner.endpoint_exts.get(port).unwrap().endpoint;
            let token = Port::to_token(port);
            self.inner
                .messenger_state
                .poll
                .register(endpoint, token, Ready::readable(), PollOpt::edge())
                .expect("AAGAGGGGG");
            self.ports.insert(port);
            port
        };
        let [kp, kg] = [clos(a, Putter), clos(b, Getter)];
        log!(
            &mut self.inner.logger,
            "!! MonoContext callback to new_channel. returning ports {:?}!",
            [kp, kg],
        );
        [kp, kg]
    }
    fn new_random(&mut self) -> u64 {
        type Bytes8 = [u8; std::mem::size_of::<u64>()];
        let mut bytes = Bytes8::default();
        getrandom::getrandom(&mut bytes).unwrap();
        let val = unsafe { std::mem::transmute::<Bytes8, _>(bytes) };
        log!(
            &mut self.inner.logger,
            "!! MonoContext callback to new_random. returning val {:?}!",
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
        logger: &mut String,
        subtree_id: SubtreeId,
        predicate: Predicate,
    ) {
        log!(logger, "NEW COMPONENT SOLUTION {:?} {:?}", subtree_id, &predicate);
        let index = self.subtree_id_to_index[&subtree_id];
        let left = 0..index;
        let right = (index + 1)..self.subtree_solutions.len();

        let Self { subtree_solutions, new_local, old_local, .. } = self;
        let was_new = subtree_solutions[index].insert(predicate.clone());
        if was_new {
            let set_visitor = left.chain(right).map(|index| &subtree_solutions[index]);
            Self::elaborate_into_new_local_rec(
                logger,
                predicate,
                set_visitor,
                old_local,
                new_local,
            );
        }
    }

    fn elaborate_into_new_local_rec<'a, 'b>(
        logger: &mut String,
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
                        logger,
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
                log!(logger, "... storing NEW LOCAL SOLUTION {:?}", &partial);
                new_local.insert(partial);
            }
        }
    }
}
impl PolyContext for BranchPContext<'_, '_> {
    type D = ProtocolD;

    fn is_firing(&mut self, port: Port) -> Option<bool> {
        assert!(self.ports.contains(&port));
        let channel_id = self.m_ctx.inner.endpoint_exts.get(port).unwrap().info.channel_id;
        let val = self.predicate.query(channel_id);
        log!(
            &mut self.m_ctx.inner.logger,
            "!! PolyContext callback to is_firing by {:?}! returning {:?}",
            self.m_ctx.my_subtree_id,
            val,
        );
        val
    }
    fn read_msg(&mut self, port: Port) -> Option<&Payload> {
        assert!(self.ports.contains(&port));
        let val = self.inbox.get(&port);
        log!(
            &mut self.m_ctx.inner.logger,
            "!! PolyContext callback to read_msg by {:?}! returning {:?}",
            self.m_ctx.my_subtree_id,
            val,
        );
        val
    }
}
