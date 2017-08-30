#![feature(conservative_impl_trait)]

extern crate futures;
extern crate tokio_core;
extern crate net2;
extern crate scheduler;
extern crate hyper;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_yaml;
extern crate zip;
extern crate percent_encoding;
#[macro_use]
extern crate lazy_static;
extern crate num_cpus;
extern crate bytes;

mod data;
mod http;
mod router;
mod request;
mod api;
mod database;

use std::fs::File;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::thread;

use tokio_core::reactor::Core;
use tokio_core::net::TcpListener;
use futures::stream::Stream;
use futures::future;
use net2::TcpBuilder;
use net2::unix::UnixTcpBuilderExt;
use hyper::server::Http;

use database::Database;
use api::Api;
use http::TravelsServer;
use data::Timestamp;

const PRIORITY_MAX: i32 = 19;

lazy_static! {
    pub static ref NOW: Timestamp = {
        use std::io::{BufRead, BufReader};
        use std::fs::File;

        File::open("/tmp/data/options.txt")
            .map(BufReader::new)
            .and_then(|mut file| {
                let mut line = String::new();
                file.read_line(&mut line)
                    .map(move |_| line)
            })
            .and_then(|line| {
                use std::io;
                line.trim()
                    .parse::<Timestamp>()
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            })
            .unwrap_or_else(|e| {
                println!("Unable to read timestamp from options.txt: {}", e);
                use std::time::{SystemTime, UNIX_EPOCH};
            
                let current_timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs() as Timestamp;
                current_timestamp
            })
    };
}

#[derive(Serialize, Deserialize)]
struct Config {
    bind:        SocketAddr,
    data_file:   String,
    keep_alive:  bool,
    num_threads: Option<usize>
}

impl Default for Config {
    fn default() -> Self {
        use std::net::{IpAddr, Ipv4Addr};
        let ip = Ipv4Addr::new(0, 0, 0, 0);
        let ip = IpAddr::V4(ip);
        let port = 80;
        let address = SocketAddr::new(ip, port);
        
        Config {
            bind: address,
            data_file: "/tmp/data/data.zip".to_string(),
            keep_alive: true,
            num_threads: Some(4)
        }
    }
}

fn main() {
    scheduler::set_self_priority(scheduler::Which::Process, PRIORITY_MAX)
        .expect("Unable to increase process priority");

    println!("Current timestamp is: {}", *NOW);
    let config: Config = File::open("config.yml")
            .map_err(|e| serde_yaml::Error::io(e))
            .and_then(serde_yaml::from_reader)
            .unwrap_or_else(|e| {
                println!("Unable to read configuration: {}", e);
                Default::default()
            });

    let service = {
        let database = Database::from_file(&config.data_file)
            .expect("Unable to initialize database");
        println!("Users: {} Locations: {}, Visits: {}", 
                 database.users.len(),
                 database.locations.len(),
                 database.visits.len());
        
        let api = {
            let api = Api { database };
            let api = RwLock::new(api);
            Arc::new(api)
        };
        
        Arc::new(TravelsServer { api })
    };

    let nthreads = config.num_threads.unwrap_or_else(num_cpus::get);
    let mut threads = Vec::with_capacity(nthreads);
    for i in 0..nthreads {
        let service = service.clone();
        let service_factory = move || service.clone();
        let is_keep_alive = config.keep_alive;

        let address = config.bind.clone();
        let thread = thread::spawn(move || {
            scheduler::set_self_affinity(scheduler::CpuSet::single(i))
                .expect("Failed to set affinity");
            
            let mut core = Core::new().expect("Failed to initialize Core");
            let listener = TcpBuilder::new_v4().expect("Failed to initialize TcpBuilder")
                .reuse_port(true).expect("Failed to reuse port")
                .bind(address).expect("Failed to bind")
                .listen(10000).expect("Failed to listen");

            let address = listener.local_addr()
                .expect("Failed to get address");

            let handle = core.handle();
            let listener = TcpListener::from_listener(listener, &address, &handle)
                .expect("Failed to initialize tcp listener");
            
            let mut http = Http::new();
            http.keep_alive(is_keep_alive);

            let server = listener.incoming().for_each(move |(socket, address)| {
                socket.set_nodelay(true).expect("Failed to set 'TCP_NODELAY' option");
                http.bind_connection(&handle, socket, address, service_factory());
                future::ok(())
            });

            core.run(server).expect("Server error");
        });
        threads.push(thread);
    }

    println!("Server started on {} ({} threads)", config.bind, nthreads);
    for thread in threads {
        thread.join().expect("Thread panic");
    }
}
