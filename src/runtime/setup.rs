use crate::common::*;
use crate::runtime::{
    actors::{MonoN, MonoP},
    endpoint::*,
    errors::*,
    *,
};

#[derive(Debug)]
enum EndpointExtTodo {
    Finished(EndpointExt),
    ActiveConnecting { addr: SocketAddr, polarity: Polarity, stream: TcpStream },
    ActiveRecving { addr: SocketAddr, polarity: Polarity, endpoint: Endpoint },
    PassiveAccepting { addr: SocketAddr, info: EndpointInfo, listener: TcpListener },
    PassiveConnecting { addr: SocketAddr, info: EndpointInfo, stream: TcpStream },
}

///////////////////// IMPL /////////////////////
impl Controller {
    // Given port bindings and a protocol config, create a connector with 1 native node
    pub fn connect(
        major: ControllerId,
        main_component: &[u8],
        protocol_description: Arc<ProtocolD>,
        bound_proto_interface: &[(PortBinding, Polarity)],
        deadline: Instant,
    ) -> Result<(Self, Vec<(Key, Polarity)>), ConnectErr> {
        use ConnectErr::*;

        let mut logger = String::default();
        log!(&mut logger, "CONNECT PHASE START! MY CID={:?} STARTING LOGGER ~", major);

        let mut channel_id_stream = ChannelIdStream::new(major);
        let mut endpoint_ext_todos = Arena::default();

        let mut ekeys_native = vec![];
        let mut ekeys_proto = vec![];
        let mut ekeys_network = vec![];

        let mut native_interface = vec![];

        /*
        1.  - allocate an EndpointExtTodo for every native and interface port
            - store all the resulting keys in two keylists for the interfaces of the native and proto components
                native: [a, c,    f]
                         |  |     |
                         |  |     |
                proto:  [b, d, e, g]
                               ^todo
                arena: <A,B,C,D,E,F,G>
        */
        for &(binding, polarity) in bound_proto_interface.iter() {
            match binding {
                PortBinding::Native => {
                    let channel_id = channel_id_stream.next();
                    let ([ekey_native, ekey_proto], native_polarity) = {
                        let [p, g] = Endpoint::new_memory_pair();
                        let mut endpoint_to_key = |endpoint, polarity| {
                            endpoint_ext_todos.alloc(EndpointExtTodo::Finished(EndpointExt {
                                endpoint,
                                info: EndpointInfo { polarity, channel_id },
                            }))
                        };
                        let pkey = endpoint_to_key(p, Putter);
                        let gkey = endpoint_to_key(g, Getter);
                        let key_pair = match polarity {
                            Putter => [gkey, pkey],
                            Getter => [pkey, gkey],
                        };
                        (key_pair, !polarity)
                    };
                    native_interface.push((ekey_native, native_polarity));
                    ekeys_native.push(ekey_native);
                    ekeys_proto.push(ekey_proto);
                }
                PortBinding::Passive(addr) => {
                    let channel_id = channel_id_stream.next();
                    let ekey_proto = endpoint_ext_todos.alloc(EndpointExtTodo::PassiveAccepting {
                        addr,
                        info: EndpointInfo { polarity, channel_id },
                        listener: TcpListener::bind(&addr).map_err(|_| BindFailed(addr))?,
                    });
                    ekeys_network.push(ekey_proto);
                    ekeys_proto.push(ekey_proto);
                }
                PortBinding::Active(addr) => {
                    let ekey_proto = endpoint_ext_todos.alloc(EndpointExtTodo::ActiveConnecting {
                        addr,
                        polarity,
                        stream: TcpStream::connect(&addr).unwrap(),
                    });
                    ekeys_network.push(ekey_proto);
                    ekeys_proto.push(ekey_proto);
                }
            }
        }
        log!(&mut logger, "{:03?} setup todos...", major);

        // 2. convert the arena to Arena<EndpointExt>  and return the
        let (mut messenger_state, mut endpoint_exts) =
            Self::finish_endpoint_ext_todos(major, &mut logger, endpoint_ext_todos, deadline)?;

        let n_mono = MonoN { ekeys: ekeys_native.into_iter().collect(), result: None };
        let p_monos = vec![MonoP {
            state: protocol_description.new_main_component(main_component, &ekeys_proto),
            ekeys: ekeys_proto.into_iter().collect(),
        }];

        // 6. Become a node in a sink tree, computing {PARENT, CHILDREN} from {NEIGHBORS}
        let family = Self::setup_sink_tree_family(
            major,
            &mut logger,
            &mut endpoint_exts,
            &mut messenger_state,
            ekeys_network,
            deadline,
        )?;

        log!(&mut logger, "CONNECT PHASE END! ~");
        let inner = ControllerInner {
            family,
            messenger_state,
            channel_id_stream,
            endpoint_exts,
            mono_ps: p_monos,
            mono_n: n_mono,
            round_index: 0,
            logger,
        };
        let controller = Self {
            protocol_description,
            inner,
            ephemeral: Default::default(),
            round_histories: vec![],
        };
        Ok((controller, native_interface))
    }

    fn test_stream_connectivity(stream: &mut TcpStream) -> bool {
        use std::io::Write;
        stream.write(&[]).is_ok()
    }

    // inserts
    fn finish_endpoint_ext_todos(
        major: ControllerId,
        logger: &mut String,
        mut endpoint_ext_todos: Arena<EndpointExtTodo>,
        deadline: Instant,
    ) -> Result<(MessengerState, Arena<EndpointExt>), ConnectErr> {
        use {ConnectErr::*, EndpointExtTodo::*};

        // 1. define and setup a poller and event loop
        let edge = PollOpt::edge();
        let [ready_r, ready_w] = [Ready::readable(), Ready::writable()];
        let mut ms = MessengerState {
            poll: Poll::new().map_err(|_| PollInitFailed)?,
            events: Events::with_capacity(endpoint_ext_todos.len()),
            delayed: vec![],
            undelayed: vec![],
            polled_undrained: Default::default(),
        };

        // 2. Register all EndpointExtTodos with ms.poll. each has one of {Endpoint, TcpStream, TcpListener}
        // 3. store the keyset of EndpointExtTodos which are not Finished in `to_finish`.
        let mut to_finish = HashSet::<_>::default();
        log!(logger, "endpoint_ext_todos len {:?}", endpoint_ext_todos.len());
        for (key, t) in endpoint_ext_todos.iter() {
            let token = key.to_token();
            match t {
                ActiveRecving { .. } | PassiveConnecting { .. } => unreachable!(),
                Finished(EndpointExt { endpoint, .. }) => {
                    ms.poll.register(endpoint, token, ready_r, edge)
                }
                ActiveConnecting { stream, .. } => {
                    to_finish.insert(key);
                    ms.poll.register(stream, token, ready_w, edge)
                }
                PassiveAccepting { listener, .. } => {
                    to_finish.insert(key);
                    ms.poll.register(listener, token, ready_r, edge)
                }
            }
            .expect("register first");
        }
        // invariant: every EndpointExtTodo has one thing registered with mio

        // 4. until all in endpoint_ext_todos are Finished variant, handle events
        let mut polled_undrained_later = IndexSet::<_>::default();
        let mut backoff_millis = 10;
        while !to_finish.is_empty() {
            ms.poll_events(deadline)?;
            for event in ms.events.iter() {
                let token = event.token();
                let ekey = Key::from_token(token);
                let entry = endpoint_ext_todos.get_mut(ekey).unwrap();
                match entry {
                    Finished(_) => {
                        polled_undrained_later.insert(ekey);
                    }
                    PassiveAccepting { addr, listener, .. } => {
                        log!(logger, "{:03?} start PassiveAccepting...", major);
                        assert!(event.readiness().is_readable());
                        let (stream, _peer_addr) =
                            listener.accept().map_err(|_| AcceptFailed(*addr))?;
                        ms.poll.deregister(listener).expect("wer");
                        ms.poll.register(&stream, token, ready_w, edge).expect("3y5");
                        take_mut::take(entry, |e| {
                            assert_let![PassiveAccepting { addr, info, .. } = e => {
                                PassiveConnecting { addr, info, stream }
                            }]
                        });
                        log!(logger, "{:03?} ... end PassiveAccepting", major);
                    }
                    PassiveConnecting { addr, stream, .. } => {
                        log!(logger, "{:03?} start PassiveConnecting...", major);
                        assert!(event.readiness().is_writable());
                        if !Self::test_stream_connectivity(stream) {
                            return Err(PassiveConnectFailed(*addr));
                        }
                        ms.poll.reregister(stream, token, ready_r, edge).expect("52");
                        let mut res = Ok(());
                        take_mut::take(entry, |e| {
                            assert_let![PassiveConnecting { info, stream, .. } = e => {
                                let mut endpoint = Endpoint::from_fresh_stream(stream);
                                let msg = Msg::SetupMsg(SetupMsg::ChannelSetup { info });
                                res = endpoint.send(msg);
                                Finished(EndpointExt { info, endpoint })
                            }]
                        });
                        res?;
                        log!(logger, "{:03?} ... end PassiveConnecting", major);
                        assert!(to_finish.remove(&ekey));
                    }
                    ActiveConnecting { addr, stream, .. } => {
                        log!(logger, "{:03?} start ActiveConnecting...", major);
                        assert!(event.readiness().is_writable());
                        if Self::test_stream_connectivity(stream) {
                            // connect successful
                            log!(logger, "CONNECT SUCCESS");
                            ms.poll.reregister(stream, token, ready_r, edge).expect("52");
                            take_mut::take(entry, |e| {
                                assert_let![ActiveConnecting { stream, polarity, addr } = e => {
                                    let endpoint = Endpoint::from_fresh_stream(stream);
                                    ActiveRecving { endpoint, polarity, addr }
                                }]
                            });
                            log!(logger, ".. ok");
                        } else {
                            // connect failure. retry!
                            log!(logger, "CONNECT FAIL");
                            ms.poll.deregister(stream).expect("wt");
                            std::thread::sleep(Duration::from_millis(backoff_millis));
                            backoff_millis = ((backoff_millis as f32) * 1.2) as u64 + 3;
                            let mut new_stream = TcpStream::connect(addr).unwrap();
                            ms.poll.register(&new_stream, token, ready_w, edge).expect("PAC 3");
                            std::mem::swap(stream, &mut new_stream);
                        }
                        log!(logger, "{:03?} ... end ActiveConnecting", major);
                    }
                    ActiveRecving { addr, polarity, endpoint } => {
                        log!(logger, "{:03?} start ActiveRecving...", major);
                        assert!(event.readiness().is_readable());
                        'recv_loop: while let Some(msg) = endpoint.recv()? {
                            if let Msg::SetupMsg(SetupMsg::ChannelSetup { info }) = msg {
                                if info.polarity == *polarity {
                                    return Err(PolarityMatched(*addr));
                                }
                                take_mut::take(entry, |e| {
                                    assert_let![ActiveRecving { polarity, endpoint, .. } = e => {
                                        let info = EndpointInfo { polarity, channel_id: info.channel_id };
                                        Finished(EndpointExt { info, endpoint })
                                    }]
                                });
                                ms.polled_undrained.insert(ekey);
                                assert!(to_finish.remove(&ekey));
                                break 'recv_loop;
                            } else {
                                ms.delayed.push(ReceivedMsg { recipient: ekey, msg });
                            }
                        }
                        log!(logger, "{:03?} ... end ActiveRecving", major);
                    }
                }
            }
        }
        for ekey in polled_undrained_later {
            ms.polled_undrained.insert(ekey);
        }
        let endpoint_exts = endpoint_ext_todos.type_convert(|(_, todo)| match todo {
            Finished(endpoint_ext) => endpoint_ext,
            _ => unreachable!(),
        });
        Ok((ms, endpoint_exts))
    }

    fn setup_sink_tree_family(
        major: ControllerId,
        logger: &mut String,
        endpoint_exts: &mut Arena<EndpointExt>,
        messenger_state: &mut MessengerState,
        neighbors: Vec<Key>,
        deadline: Instant,
    ) -> Result<ControllerFamily, ConnectErr> {
        use {ConnectErr::*, Msg::SetupMsg as S, SetupMsg::*};

        log!(logger, "neighbors {:?}", &neighbors);

        let mut messenger = (messenger_state, endpoint_exts);
        impl Messengerlike for (&mut MessengerState, &mut Arena<EndpointExt>) {
            fn get_state_mut(&mut self) -> &mut MessengerState {
                self.0
            }
            fn get_endpoint_mut(&mut self, ekey: Key) -> &mut Endpoint {
                &mut self.1.get_mut(ekey).expect("OUT OF BOUNDS").endpoint
            }
        }

        // 1. broadcast my ID as the first echo. await reply from all in net_keylist
        let echo = S(LeaderEcho { maybe_leader: major });
        let mut awaiting = IndexSet::with_capacity(neighbors.len());
        for &n in neighbors.iter() {
            log!(logger, "{:?}'s initial echo to {:?}, {:?}", major, n, &echo);
            messenger.send(n, echo.clone())?;
            awaiting.insert(n);
        }

        // 2. Receive incoming replies. whenever a higher-id echo arrives,
        //    adopt it as leader, sender as parent, and reset the await set.
        let mut parent: Option<Key> = None;
        let mut my_leader = major;
        messenger.undelay_all();
        'echo_loop: while !awaiting.is_empty() || parent.is_some() {
            let ReceivedMsg { recipient, msg } = messenger.recv(deadline)?.ok_or(Timeout)?;
            log!(logger, "{:?} GOT {:?} {:?}", major, &recipient, &msg);
            match msg {
                S(LeaderAnnounce { leader }) => {
                    // someone else completed the echo and became leader first!
                    // the sender is my parent
                    parent = Some(recipient);
                    my_leader = leader;
                    awaiting.clear();
                    break 'echo_loop;
                }
                S(LeaderEcho { maybe_leader }) => {
                    use Ordering::*;
                    match maybe_leader.cmp(&my_leader) {
                        Less => { /* ignore */ }
                        Equal => {
                            awaiting.remove(&recipient);
                            if awaiting.is_empty() {
                                if let Some(p) = parent {
                                    // return the echo to my parent
                                    messenger.send(p, S(LeaderEcho { maybe_leader }))?;
                                } else {
                                    // DECIDE!
                                    break 'echo_loop;
                                }
                            }
                        }
                        Greater => {
                            // join new echo
                            log!(logger, "{:?} setting leader to {:?}", major, recipient);
                            parent = Some(recipient);
                            my_leader = maybe_leader;
                            let echo = S(LeaderEcho { maybe_leader: my_leader });
                            awaiting.clear();
                            if neighbors.len() == 1 {
                                // immediately reply to parent
                                log!(
                                    logger,
                                    "{:?} replying echo to parent {:?} immediately",
                                    major,
                                    recipient
                                );
                                messenger.send(recipient, echo.clone())?;
                            } else {
                                for &n in neighbors.iter() {
                                    if n != recipient {
                                        log!(
                                            logger,
                                            "{:?} repeating echo {:?} to {:?}",
                                            major,
                                            &echo,
                                            n
                                        );
                                        messenger.send(n, echo.clone())?;
                                        awaiting.insert(n);
                                    }
                                }
                            }
                        }
                    }
                }
                msg => messenger.delay(ReceivedMsg { recipient, msg }),
            }
        }
        match parent {
            None => assert_eq!(
                my_leader, major,
                "I've got no parent, but I consider {:?} the leader?",
                my_leader
            ),
            Some(parent) => assert_ne!(
                my_leader, major,
                "I have {:?} as parent, but I consider myself ({:?}) the leader?",
                parent, major
            ),
        }

        log!(logger, "{:?} DONE WITH ECHO! Leader has cid={:?}", major, my_leader);

        // 3. broadcast leader announcement (except to parent: confirm they are your parent)
        //    in this loop, every node sends 1 message to each neighbor
        let msg_for_non_parents = S(LeaderAnnounce { leader: my_leader });
        for &k in neighbors.iter() {
            let msg =
                if Some(k) == parent { S(YouAreMyParent) } else { msg_for_non_parents.clone() };
            log!(logger, "{:?} ANNOUNCING to {:?} {:?}", major, k, &msg);
            messenger.send(k, msg)?;
        }

        // await 1 message from all non-parents
        for &n in neighbors.iter() {
            if Some(n) != parent {
                awaiting.insert(n);
            }
        }
        let mut children = Vec::default();
        messenger.undelay_all();
        while !awaiting.is_empty() {
            let ReceivedMsg { recipient, msg } = messenger.recv(deadline)?.ok_or(Timeout)?;
            match msg {
                S(YouAreMyParent) => {
                    assert!(awaiting.remove(&recipient));
                    children.push(recipient);
                }
                S(SetupMsg::LeaderAnnounce { leader }) => {
                    assert!(awaiting.remove(&recipient));
                    assert!(leader == my_leader);
                    assert!(Some(recipient) != parent);
                    // they wouldn't send me this if they considered me their parent
                }
                _ => messenger.delay(ReceivedMsg { recipient, msg }),
            }
        }
        Ok(ControllerFamily { parent_ekey: parent, children_ekeys: children })
    }
}

impl Messengerlike for Controller {
    fn get_state_mut(&mut self) -> &mut MessengerState {
        &mut self.inner.messenger_state
    }
    fn get_endpoint_mut(&mut self, ekey: Key) -> &mut Endpoint {
        &mut self.inner.endpoint_exts.get_mut(ekey).expect("OUT OF BOUNDS").endpoint
    }
}
