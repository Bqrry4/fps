extern crate enet;

use enet::*;
use std::{fs::remove_dir, future::pending, io::Read, mem, net::Ipv4Addr, pin::Pin, sync::atomic::{AtomicI32, Ordering}};

use crate::dto;
use dto::PlayerInfo;

const ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;
const PORT: u16 = 6969;

pub struct NetworkClient {
    host: Pin<Box<Host<()>>>,
    peer: Option<Peer<'static, ()>>,
    pub remotePlayers: Vec<PlayerInfo>,
}

// static mut CLIENT_ID: i32 = -1;
pub static CLIENT_ID: AtomicI32 = AtomicI32::new(-1);

impl NetworkClient {

    pub fn new() -> Result<Self, String> {
        let enet = Enet::new().map_err(|_| "Could not initialize ENet")?;

        let host = enet
            .create_host::<()>(
                None,
                1,
                ChannelLimit::Maximum,
                BandwidthLimit::Unlimited,
                BandwidthLimit::Unlimited,
            )
            .map_err(|_| "Could not create host")?;

        Ok(Self {
            host: Box::pin(host),
            peer: None,
            remotePlayers: Vec::new()
        })
    }

    pub fn connect(&mut self) -> Result<(), String> {
        let host_ref: &'static mut Host<()> = unsafe { std::mem::transmute(&mut *self.host) };

        let peer = host_ref
            .connect(&Address::new(ADDRESS, PORT), 1, 0)
            .map_err(|_| "connect failed")?;

        self.peer = Some(peer);

        //Wait for the assigned id
        loop{
            match host_ref.service(0).map_err(|_| "connect failed")? {
                Some(Event::Receive { ref packet, .. }) => {
                    if packet.data().len() == 4 {
                        CLIENT_ID.store(i32::from_le_bytes(packet.data()[0..4].try_into().expect("slice with incorrect length")), Ordering::SeqCst);
                        // unsafe { CLIENT_ID = i32::from_le_bytes(packet.data()[0..4].try_into().expect("slice with incorrect length")) };
                    } else {
                        return Err("Invalid packet length".to_string());
                    }
                    break;
                }
                Some(Event::Connect(..)) => {}
                Some(Event::Disconnect(..)) => {}
                None => {}
            }
        }
        println!("{}", CLIENT_ID.load(Ordering::SeqCst));

        Ok(())
    }

    pub fn update(&mut self) {
        let e = self.host.service(0).unwrap();
        if let Some(e) = &e {
            match e{
                Event::Receive { packet, .. } => {
                    let info: PlayerInfo = bincode::deserialize(packet.data()).unwrap();

                    if let Some(i) = self.remotePlayers.iter().position(|x| x.id == info.id) {
                        // Update existing player
                        self.remotePlayers[i] = info;
                    } else {
                        // Add new player
                        self.remotePlayers.push(info);
                    }
                }

                //Ignore these for now
                Event::Connect { .. } => {}
                Event::Disconnect { .. } => {}
            }
        }
    }

    pub fn send_update(&mut self, info: PlayerInfo) {
        if let Some(peer) = &mut self.peer {
            let bytes: Vec<u8> = bincode::serialize(&info).expect("serialization failed");
            let packet = Packet::new(&bytes, PacketMode::UnreliableSequenced)
                .expect("packet creation failed");
            let _ = peer.send_packet(packet, 0);
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(peer) = &mut self.peer {
            peer.disconnect(0);
            self.peer = None;
        }
    }

    pub fn is_connected(&self) -> bool {
        self.peer.is_some()
    }
}
