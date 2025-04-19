extern crate enet;

use enet::*;
use std::{mem, net::Ipv4Addr};

const ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;
const PORT: u16 = 6969;

pub struct NetworkClient {
    host: Host<()>,
    peer: Option<Peer<'static, ()>>,
}

impl NetworkClient {
    pub fn try_(&mut self) -> Result<(), String> {
        let mut peer = self
            .host
            .connect(&Address::new(Ipv4Addr::LOCALHOST, 6969), 1, 0)
            .map_err(|_| "connect failed")?;

        // loop {
        //     let e = self.host.service(1000).map_err(|_|"service failed")?;
        //     println!("received event: {:#?}", e);
        // }
        Ok(())
    }

    pub fn new() -> Result<Self, String> {
        let enet = Enet::new().map_err(|_| "Could not initialize ENet")?;

        let mut host = enet
            .create_host::<()>(
                None,
                1,
                ChannelLimit::Maximum,
                BandwidthLimit::Unlimited,
                BandwidthLimit::Unlimited,
            )
            .map_err(|_| "Could not create host")?;

        Ok(Self { host, peer: None })
    }

    pub fn connect(&mut self) -> Result<(), String> {
        self.host
            .connect(&Address::new(Ipv4Addr::LOCALHOST, 9001), 10, 0)
            .map_err(|_| "connect failed")?;

        let mut peer = loop {
            let e = self.host.service(1000).map_err(|_| "service failed")?;

            let e = match e {
                Some(ev) => ev,
                _ => continue,
            };

            println!("[client] event: {:#?}", e);

            match e {
                Event::Connect(ref p) => {
                    break p.clone();
                }
                Event::Disconnect(ref p, r) => {
                    println!("connection NOT successful, peer: {:?}, reason: {}", p, r);
                    std::process::exit(0);
                }
                Event::Receive { .. } => {}
            };
        };

        // borrow the pinned host to call `connect()`
        // let mut pinned_host = Pin::new(&mut self.host);
        // let mut binding = pinned_host
        //     .as_mut();
        // let short_lived: enet::Peer<'_, ()> = binding
        //     .connect(&Address::new(ADDRESS, PORT), 1, 0)
        //     .map_err(|_| "connect failed")?;

        // **SAFETY**: We promise that
        // 1) `host` lives for the entire `NetworkClient`,
        // 2) `peer` is always dropped *before* `host` (field order),
        // so extending this borrow to `'static` is sound.
        let long: enet::Peer<'static, ()> = unsafe { mem::transmute(peer) };
        self.peer = Some(long);
        Ok(())
    }

    pub fn update(&mut self) {
        let e = self.host.service(0).unwrap();
        if let Some(e) = &e{
            println!("received event: {:#?}", e);
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
