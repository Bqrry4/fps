extern crate enet;

use std::net::Ipv4Addr;

use anyhow::Context;
use enet::*;
mod dto;
use dto::PlayerInfo;

const MAX_PEERS: usize = 5;
const PORT: u16 = 6969;

pub struct PlayerDetails<'a> {
    peer: Peer<'a, ()>,
}

fn main() -> anyhow::Result<()> {
    let enet = Enet::new().context("could not initialize ENet")?;
    let addr = Address::new(Ipv4Addr::LOCALHOST, PORT);
    let mut host = enet
        .create_host::<()>(
            Some(&addr),
            MAX_PEERS,
            ChannelLimit::Maximum,
            BandwidthLimit::Unlimited,
            BandwidthLimit::Unlimited,
        )
        .context("could not create host")?;

    let mut players = Vec::with_capacity(MAX_PEERS);

    loop {
        if let Some(event) = host.service(1000).context("service failed")? {
            handle_event(event, &mut players);
        }
    }
}


static mut id: i32 = 0;

fn handle_event<'a>(event: Event<'a, ()>, players: &mut Vec<PlayerDetails<'a>>) {
    match event {
        Event::Connect(ref peer) => {
            let mut peer_clone = peer.clone();
            let id_bytes: [u8; 4] = unsafe { id.to_le_bytes() };
            let _ = peer_clone.send_packet(Packet::new(id_bytes.as_ref(), PacketMode::ReliableSequenced).unwrap(), 0);
            players.push(PlayerDetails {
                peer: peer_clone,
            });
            println!("new connection: {}", players.len());
            unsafe { id += 1 };
        }

        Event::Disconnect(ref peer, _data) => {
            // Use `retain` to avoid separate mutable borrow during an iteration
            let addr = peer.address();
            players.retain(|p| p.peer.address() != addr);
            println!("disconnected: {}", players.len());
        }

        Event::Receive {
            ref packet,
            ref sender,
            ..
        } => {

            players
                .iter_mut()
                .filter(|p| p.peer.address() != sender.address())
                .for_each(|p| {
                    let packet = Packet::new(packet.data(), PacketMode::UnreliableSequenced).unwrap();
                    p.peer.send_packet(packet, 0).expect("failed to send packet tp peer"); 
                });
        }
    }
}


