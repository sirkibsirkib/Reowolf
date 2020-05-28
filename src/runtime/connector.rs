use crate::common::*;
use crate::runtime::{errors::*, *};

pub fn random_controller_id() -> ControllerId {
    type Bytes8 = [u8; std::mem::size_of::<ControllerId>()];
    let mut bytes = Bytes8::default();
    getrandom::getrandom(&mut bytes).unwrap();
    unsafe { std::mem::transmute::<Bytes8, ControllerId>(bytes) }
}

impl Default for Unconfigured {
    fn default() -> Self {
        let controller_id = random_controller_id();
        Self { controller_id }
    }
}
impl Default for Connector {
    fn default() -> Self {
        Self::Unconfigured(Unconfigured::default())
    }
}
impl Connector {
    /// Configure the Connector with the given Pdl description.
    pub fn configure(&mut self, pdl: &[u8], main_component: &[u8]) -> Result<(), ConfigErr> {
        use ConfigErr::*;
        let controller_id = match self {
            Connector::Configured(_) => return Err(AlreadyConfigured),
            Connector::Connected(_) => return Err(AlreadyConnected),
            Connector::Unconfigured(Unconfigured { controller_id }) => *controller_id,
        };
        let protocol_description = Arc::new(ProtocolD::parse(pdl).map_err(ParseErr)?);
        let polarities = protocol_description.component_polarities(main_component)?;
        let configured = Configured {
            controller_id,
            protocol_description,
            bindings: Default::default(),
            polarities,
            main_component: main_component.to_vec(),
            logger: "Logger created!\n".into(),
        };
        *self = Connector::Configured(configured);
        Ok(())
    }

    /// Bind the (configured) connector's port corresponding to the
    pub fn bind_port(
        &mut self,
        proto_port_index: usize,
        binding: PortBinding,
    ) -> Result<(), PortBindErr> {
        use PortBindErr::*;
        match self {
            Connector::Unconfigured { .. } => Err(NotConfigured),
            Connector::Connected(_) => Err(AlreadyConnected),
            Connector::Configured(configured) => {
                if configured.polarities.len() <= proto_port_index {
                    return Err(IndexOutOfBounds);
                }
                configured.bindings.insert(proto_port_index, binding);
                Ok(())
            }
        }
    }
    pub fn connect(&mut self, timeout: Duration) -> Result<(), ConnectErr> {
        let deadline = Instant::now() + timeout;
        use ConnectErr::*;
        let configured = match self {
            Connector::Unconfigured { .. } => return Err(NotConfigured),
            Connector::Connected(_) => return Err(AlreadyConnected),
            Connector::Configured(configured) => configured,
        };
        // 1. Unwrap bindings or err
        let bound_proto_interface: Vec<(_, _)> = configured
            .polarities
            .iter()
            .copied()
            .enumerate()
            .map(|(native_index, polarity)| {
                let binding = configured
                    .bindings
                    .get(&native_index)
                    .copied()
                    .ok_or(PortNotBound { native_index })?;
                Ok((binding, polarity))
            })
            .collect::<Result<Vec<(_, _)>, ConnectErr>>()?;
        let (controller, native_interface) = Controller::connect(
            configured.controller_id,
            &configured.main_component,
            configured.protocol_description.clone(),
            &bound_proto_interface[..],
            &mut configured.logger,
            deadline,
        )?;
        *self = Connector::Connected(Connected {
            native_interface,
            sync_batches: vec![Default::default()],
            controller,
        });
        Ok(())
    }
    pub fn get_mut_logger(&mut self) -> Option<&mut String> {
        match self {
            Connector::Configured(configured) => Some(&mut configured.logger),
            Connector::Connected(connected) => Some(&mut connected.controller.inner.logger),
            _ => None,
        }
    }

    pub fn put(&mut self, native_port_index: usize, payload: Payload) -> Result<(), PortOpErr> {
        use PortOpErr::*;
        let connected = match self {
            Connector::Connected(connected) => connected,
            _ => return Err(NotConnected),
        };
        let (port, native_polarity) =
            *connected.native_interface.get(native_port_index).ok_or(IndexOutOfBounds)?;
        if native_polarity != Putter {
            return Err(WrongPolarity);
        }
        let sync_batch = connected.sync_batches.iter_mut().last().expect("no sync batch!");
        if sync_batch.puts.contains_key(&port) {
            return Err(DuplicateOperation);
        }
        sync_batch.puts.insert(port, payload);
        Ok(())
    }

    pub fn get(&mut self, native_port_index: usize) -> Result<(), PortOpErr> {
        use PortOpErr::*;
        let connected = match self {
            Connector::Connected(connected) => connected,
            _ => return Err(NotConnected),
        };
        let (port, native_polarity) =
            *connected.native_interface.get(native_port_index).ok_or(IndexOutOfBounds)?;
        if native_polarity != Getter {
            return Err(WrongPolarity);
        }
        let sync_batch = connected.sync_batches.iter_mut().last().expect("no sync batch!");
        if sync_batch.gets.contains(&port) {
            return Err(DuplicateOperation);
        }
        sync_batch.gets.insert(port);
        Ok(())
    }
    pub fn next_batch(&mut self) -> Result<usize, ()> {
        let connected = match self {
            Connector::Connected(connected) => connected,
            _ => return Err(()),
        };
        connected.sync_batches.push(SyncBatch::default());
        Ok(connected.sync_batches.len() - 2)
    }

    pub fn sync(&mut self, timeout: Duration) -> Result<usize, SyncErr> {
        let deadline = Instant::now() + timeout;
        use SyncErr::*;
        let connected = match self {
            Connector::Connected(connected) => connected,
            _ => return Err(NotConnected),
        };

        // do the synchronous round!
        let res =
            connected.controller.sync_round(Some(deadline), Some(connected.sync_batches.drain(..)));
        connected.sync_batches.push(SyncBatch::default());
        res?;
        Ok(connected.controller.inner.mono_n.result.as_mut().expect("qqqs").0)
    }

    pub fn read_gotten(&self, native_port_index: usize) -> Result<&[u8], ReadGottenErr> {
        use ReadGottenErr::*;
        let connected = match self {
            Connector::Connected(connected) => connected,
            _ => return Err(NotConnected),
        };
        let &(key, polarity) =
            connected.native_interface.get(native_port_index).ok_or(IndexOutOfBounds)?;
        if polarity != Getter {
            return Err(WrongPolarity);
        }
        let result = connected.controller.inner.mono_n.result.as_ref().ok_or(NoPreviousRound)?;
        let payload = result.1.get(&key).ok_or(DidNotGet)?;
        Ok(payload.as_slice())
    }
}
