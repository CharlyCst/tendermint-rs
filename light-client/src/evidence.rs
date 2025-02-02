//! Fork evidence data structures and interfaces.

use crate::{components::io::IoError, types::PeerId};

use tendermint::abci::transaction::Hash;

use contracts::contract_trait;

pub use tendermint::evidence::Evidence;

/// Interface for reporting evidence to full nodes, typically via the RPC client.
#[contract_trait]
#[allow(missing_docs)] // This is required because of the `contracts` crate (TODO: open/link issue)
pub trait EvidenceReporter: Send {
    /// Report evidence to all connected full nodes.
    fn report(&self, e: Evidence, peer: PeerId) -> Result<Hash, IoError>;
}

#[cfg(feature = "rpc-client")]
pub use self::prod::ProdEvidenceReporter;

#[cfg(feature = "rpc-client")]
mod prod {
    use super::*;
    use crate::utils::block_on;

    use contracts::pre;
    use std::collections::HashMap;

    use tendermint_rpc as rpc;
    use tendermint_rpc::Client;

    /// Production implementation of the EvidenceReporter component, which reports evidence to full
    /// nodes via RPC.
    #[derive(Clone, Debug)]
    pub struct ProdEvidenceReporter {
        peer_map: HashMap<PeerId, tendermint::net::Address>,
    }

    #[contract_trait]
    impl EvidenceReporter for ProdEvidenceReporter {
        #[pre(self.peer_map.contains_key(&peer))]
        fn report(&self, e: Evidence, peer: PeerId) -> Result<Hash, IoError> {
            let client = self.rpc_client_for(peer)?;
            let task = async move { client.broadcast_evidence(e).await };
            let res = block_on(task, peer, None)?;

            match res {
                Ok(response) => Ok(response.hash),
                Err(err) => Err(IoError::RpcError(err)),
            }
        }
    }

    impl ProdEvidenceReporter {
        /// Constructs a new ProdEvidenceReporter component.
        ///
        /// A peer map which maps peer IDS to their network address must be supplied.
        pub fn new(peer_map: HashMap<PeerId, tendermint::net::Address>) -> Self {
            Self { peer_map }
        }

        #[pre(self.peer_map.contains_key(&peer))]
        fn rpc_client_for(&self, peer: PeerId) -> Result<rpc::HttpClient, IoError> {
            let peer_addr = self.peer_map.get(&peer).unwrap().to_owned();
            Ok(rpc::HttpClient::new(peer_addr).map_err(IoError::from)?)
        }
    }
}
