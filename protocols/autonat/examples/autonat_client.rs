// Copyright 2021 Protocol Labs.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Basic example that combines the AutoNAT and identify protocols.
//!
//! The identify protocol informs the local peer of its external addresses, that are then send in AutoNAT dial-back
//! requests to the server.
//!
//! To run this example, follow the instructions in `examples/server` to start a server, then run in a new terminal:
//! ```sh
//! cargo run --example client -- --server-address <server-addr> --server-peer-id <server-peer-id> --listen-port <port>
//! ```
//! The `listen-port` parameter is optional and allows to set a fixed port at which the local client should listen.

use clap::Parser;
use futures::prelude::*;
use libp2p::autonat;
use libp2p::identify;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use libp2p::{identity, Multiaddr, PeerId};
use std::error::Error;
use std::net::Ipv4Addr;
use std::time::Duration;

#[derive(Debug, Parser)]
#[clap(name = "libp2p autonat")]
struct Opt {
    #[clap(long)]
    listen_port: Option<u16>,

    #[clap(long)]
    server_address: Multiaddr,

    #[clap(long)]
    server_peer_id: PeerId,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let opt = Opt::parse();

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {:?}", local_peer_id);

    let transport = libp2p::development_transport(local_key.clone()).await?;

    let behaviour = Behaviour::new(local_key.public());

    let mut swarm = Swarm::with_async_std_executor(transport, behaviour, local_peer_id);
    swarm.listen_on(
        Multiaddr::empty()
            .with(Protocol::Ip4(Ipv4Addr::UNSPECIFIED))
            .with(Protocol::Tcp(opt.listen_port.unwrap_or(0))),
    )?;

    swarm
        .behaviour_mut()
        .auto_nat
        .add_server(opt.server_peer_id, Some(opt.server_address));

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => println!("Listening on {:?}", address),
            SwarmEvent::Behaviour(event) => println!("{:?}", event),
            e => println!("{:?}", e),
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "Event")]
struct Behaviour {
    identify: identify::Behaviour,
    auto_nat: autonat::Behaviour,
}

impl Behaviour {
    fn new(local_public_key: identity::PublicKey) -> Self {
        Self {
            identify: identify::Behaviour::new(identify::Config::new(
                "/ipfs/0.1.0".into(),
                local_public_key.clone(),
            )),
            auto_nat: autonat::Behaviour::new(
                local_public_key.to_peer_id(),
                autonat::Config {
                    retry_interval: Duration::from_secs(10),
                    refresh_interval: Duration::from_secs(30),
                    boot_delay: Duration::from_secs(5),
                    throttle_server_period: Duration::ZERO,
                    ..Default::default()
                },
            ),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum Event {
    AutoNat(autonat::Event),
    Identify(identify::Event),
}

impl From<identify::Event> for Event {
    fn from(v: identify::Event) -> Self {
        Self::Identify(v)
    }
}

impl From<autonat::Event> for Event {
    fn from(v: autonat::Event) -> Self {
        Self::AutoNat(v)
    }
}
