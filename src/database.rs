use std::collections::{HashMap, BTreeMap};
use std::error::Error;
use std::path::Path;
use std::fs::File;
use std::fmt::Display;
use std::io::Read;

use serde_json;
use zip::ZipArchive;

use data::*;

#[derive(Default)]
pub struct Database {
    pub users: HashMap<UserId, User>,
    pub locations: HashMap<LocationId, Location>,
    pub visits: HashMap<VisitId, Visit>,
    
    // for /user/<id>/visits request
    pub visits_by_user: HashMap<UserId, BTreeMap<Timestamp, Visit>>,
    
    // for /locations/<id>/avg request
    pub visits_by_location: HashMap<LocationId, BTreeMap<Timestamp, Visit>>
}

impl Database {
    #[inline]
    pub fn from_file<P: AsRef<Path> + Display>(path: P) -> Result<Database, Box<Error>> {
        let mut database = Database::default();
        
        // info!("Loading database from {}", path);
        let zip_file = File::open(path)?;
        let mut archive = ZipArchive::new(zip_file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).expect("Unable to read zip file");
            if file.name().starts_with("users") {
                #[derive(Deserialize)]
                struct Users {
                    users: Vec<User>
                }

                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let Users { users } = serde_json::from_slice(&bytes)?;
                for user in users {
                    database.users.insert(user.id, user);
                }
            } else if file.name().starts_with("locations") {
                #[derive(Deserialize)]
                struct Locations {
                    locations: Vec<Location>
                }

                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let Locations { locations } = serde_json::from_slice(&bytes)?;
                for location in locations {
                    database.locations.insert(location.id, location);
                }
            } else if file.name().starts_with("visits") {
                #[derive(Deserialize)]
                struct Visits {
                    visits: Vec<Visit>
                }

                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let Visits { visits } = serde_json::from_slice(&bytes)?;
                for visit in visits {
                    database.visits.insert(visit.id, visit.clone());
                    database.visits_by_location.entry(visit.location)
                        .or_insert_with(Default::default)
                        .insert(visit.visited_at, visit.clone());
                    database.visits_by_user.entry(visit.user)
                        .or_insert_with(Default::default)
                        .insert(visit.visited_at, visit.clone());
                }     
            }
        }

        Ok(database)
    }
}