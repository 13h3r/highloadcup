use serde_json;
use hyper::StatusCode;
use bytes::Bytes;

use data::*;
use request::*;
use database::Database;

pub struct Api {
    pub database: Database
}

static EMPTY_VISITS_RESPONSE: &'static [u8] = b"{\"visits\":[]}";
static ZERO_AVERAGE_RESPONSE: &'static [u8] = b"{\"avg\":0}";
static POST_RESPONSE: &'static [u8] = b"{}";

impl Api {
    #[inline]
    pub fn do_post(&mut self, request: PostRequest) -> Result<Bytes, StatusCode> {
        use request::PostRequest::*;
        match request {
            UpdateEntity(update) => self.update_entity(update),
            CreateEntity(entity) => self.create_entity(entity)
        }
    }

    #[inline]
    pub fn do_get(&self, request: GetRequest) -> Result<Bytes, StatusCode> {
        use request::GetRequest::*;
        match request {
            GetEntity(entity_request) => self.get_entity(entity_request),
            GetVisits(id, parameters) => self.get_visits(id, parameters),
            GetAverageLocationRating(id, parameters) 
                => self.get_average_location_rating(id, parameters)
        }
    }

    #[inline]
    fn get_entity(&self, request: GetEntity) -> Result<Bytes, StatusCode> {
        let bytes = match request {
            GetEntity::User(id) => {
                let user = self.database.users.get(&id)
                    .ok_or(StatusCode::NotFound)?;

                serde_json::to_vec(user).unwrap()
            },
            GetEntity::Location(id) => {
                let location = self.database.locations.get(&id)
                    .ok_or(StatusCode::NotFound)?;

                serde_json::to_vec(location).unwrap()
            },
            GetEntity::Visit(id) => {
                let visit = self.database.visits.get(&id)
                    .ok_or(StatusCode::NotFound)?;

                serde_json::to_vec(visit).unwrap()
            }
        };

        Ok(bytes.into())
    }

    #[inline]
    fn get_visits(&self, id: UserId, parameters: GetVisits) -> Result<Bytes, StatusCode> {
        use std::collections::Bound::Excluded;
        if !self.database.users.contains_key(&id) {
            return Err(StatusCode::NotFound);
        }
        
        #[derive(Serialize)]
        struct VisitItem<'a> {
            mark: u8,
            visited_at: Timestamp,
            place: &'a str
        }

        #[derive(Serialize)]
        struct VisitsResponse<'a> {
            visits: Vec<VisitItem<'a>>
        }
        
        let from_date = parameters.from_date.unwrap_or(Timestamp::min_value());
        let to_date = parameters.to_date.unwrap_or(Timestamp::max_value());

        if from_date >= to_date {
            return Ok(Bytes::from_static(EMPTY_VISITS_RESPONSE));
        }

        let user_visits = match self.database.visits_by_user.get(&id) {
            Some(visits) => visits,
            None => return Ok(Bytes::from_static(EMPTY_VISITS_RESPONSE))
        };

        let mut visits = Vec::new();
        for (_visit_id, visit) in user_visits.range((Excluded(from_date), Excluded(to_date))) {
            let location = self.database.locations.get(&visit.location)
                .ok_or(StatusCode::InternalServerError)?;
            
            if parameters.to_distance.is_some() 
            && location.distance >= parameters.to_distance.unwrap() {
                    continue;
            }

            if parameters.country.is_some() 
            && location.country.as_str() != parameters.country.as_ref().unwrap() {
                continue;
            }

            let &Visit { visited_at, mark, .. } = visit;
            let place = location.place.as_str();
            visits.push(VisitItem { mark, visited_at, place });
        }

        if !visits.is_empty() {
            Ok(serde_json::to_vec(&VisitsResponse { visits }).unwrap().into())
        } else {
            Ok(Bytes::from_static(EMPTY_VISITS_RESPONSE))
        }
    }

    #[inline]
    fn get_average_location_rating(&self, id: LocationId, 
                                   parameters: GetAverageLocationRating) 
                                   -> Result<Bytes, StatusCode> 
    {
        use std::collections::Bound::Excluded;
        if !self.database.locations.contains_key(&id) {
            return Err(StatusCode::NotFound);
        }

        let visits = match self.database.visits_by_location.get(&id) {
            Some(visits) => visits,
            None => return Ok(Bytes::from_static(ZERO_AVERAGE_RESPONSE))
        };

        let needs_user_data = 
               parameters.gender.is_some() 
            || parameters.from_age.is_some() 
            || parameters.to_age.is_some();

        const SECONDS_IN_YEAR: i64 = 31557600; // 365.25 days

        let max_birth_date = parameters.from_age
            .map(|age| *::NOW - SECONDS_IN_YEAR * age)
            .unwrap_or(Timestamp::max_value());
        let min_birth_date = parameters.to_age
            .map(|age| *::NOW - SECONDS_IN_YEAR * age)
            .unwrap_or(Timestamp::min_value());

        let from_date = parameters.from_date.unwrap_or(Timestamp::min_value());
        let to_date   = parameters.to_date.unwrap_or(Timestamp::max_value());

        if from_date >= to_date || min_birth_date >= max_birth_date {
            return Ok(Bytes::from_static(ZERO_AVERAGE_RESPONSE));
        }

        let mut sum = 0usize;
        let mut count = 0;
        for (_visit_id, visit) in visits.range((Excluded(from_date), Excluded(to_date))) {
            if needs_user_data {
                let user = self.database.users.get(&visit.user)
                    .ok_or(StatusCode::InternalServerError)?;
                
                if parameters.gender.is_some() && 
                   user.gender != parameters.gender.unwrap() {
                    continue;
                }

                if user.birth_date <= min_birth_date ||
                   user.birth_date >= max_birth_date {
                    continue;
                }
            };

            sum += visit.mark as usize;
            count += 1;
        }

        if count != 0 {
            let avg = sum as f64 / count as f64;
            let avg = (avg * 100000.0).round() / 100000.0;
            // using format here because of floating point arithmetic inaccuracy
            let bytes = format!("{{\"avg\":{:.5}}}", avg).into_bytes();
            Ok(bytes.into())
        } else {
            Ok(Bytes::from_static(ZERO_AVERAGE_RESPONSE))
        }
    } 

    #[inline]
    fn update_entity(&mut self, request: UpdateEntity) -> Result<Bytes, StatusCode> {
        use request::Optional::Something;
        
        match request {
            UpdateEntity::User(id, update) => {
                let user = self.database.users.get_mut(&id)
                    .ok_or(StatusCode::NotFound)?;
                
                if let Something(email) = update.email {
                    user.email = email;
                }

                if let Something(first_name) = update.first_name {
                    user.first_name = first_name;
                }

                if let Something(last_name) = update.last_name {
                    user.last_name = last_name;
                }

                if let Something(gender) = update.gender {
                    user.gender = gender;
                }

                if let Something(birth_date) = update.birth_date {
                    user.birth_date = birth_date;
                }
            },
            UpdateEntity::Location(id, update) => {
                let location = self.database.locations.get_mut(&id)
                    .ok_or(StatusCode::NotFound)?;
                
                if let Something(place) = update.place {
                    location.place = place;
                }

                if let Something(country) = update.country {
                    location.country = country;
                }

                if let Something(city) = update.city {
                    location.city = city;
                }

                if let Something(distance) = update.distance {
                    location.distance = distance;
                }
            },
            UpdateEntity::Visit(id, update) => {
                let visit = self.database.visits.get_mut(&id)
                    .ok_or(StatusCode::NotFound)?;

                if let Something(ref location) = update.location {
                    if !self.database.locations.contains_key(location) {
                        return Err(StatusCode::BadRequest);
                    }
                }

                if let Something(ref user) = update.user {
                    if !self.database.users.contains_key(user) {
                        return Err(StatusCode::BadRequest);
                    }
                }

                if let Something(location) = update.location {
                    self.database.visits_by_location
                        .get_mut(&visit.location)
                        .ok_or(StatusCode::InternalServerError)?
                        .remove(&visit.visited_at);

                    visit.location = location;
                }

                if let Something(user) = update.user {
                    self.database.visits_by_user
                        .get_mut(&visit.user)
                        .ok_or(StatusCode::InternalServerError)?
                        .remove(&visit.visited_at);

                    visit.user = user;
                }

                if let Something(visited_at) = update.visited_at {
                    // possibly deleted in previous branch
                    self.database.visits_by_location
                        .get_mut(&visit.location)
                        .map(|visits| visits.remove(&visit.visited_at));

                    self.database.visits_by_user
                        .get_mut(&visit.user)
                        .map(|visits| visits.remove(&visit.visited_at));
                    
                    visit.visited_at = visited_at;
                }

                if let Something(mark) = update.mark {
                    visit.mark = mark;
                }

                self.database.visits_by_location
                    .entry(visit.location)
                    .or_insert_with(Default::default)
                    .insert(visit.visited_at, visit.clone());

                self.database.visits_by_user
                    .entry(visit.user)
                    .or_insert_with(Default::default)
                    .insert(visit.visited_at, visit.clone());
            }
        };

        Ok(Bytes::from_static(POST_RESPONSE))
    }

    #[inline]
    fn create_entity(&mut self, request: CreateEntity) -> Result<Bytes, StatusCode> {
        use std::collections::hash_map::Entry;

        match request {
            CreateEntity::User(user) => {
                match self.database.users.entry(user.id) {
                    Entry::Occupied(_) => return Err(StatusCode::BadRequest),
                    Entry::Vacant(v) => v.insert(user)
                };
            },
            CreateEntity::Location(location) => {
                match self.database.locations.entry(location.id) {
                    Entry::Occupied(_) => return Err(StatusCode::BadRequest),
                    Entry::Vacant(v) => v.insert(location)
                };
            },
            CreateEntity::Visit(visit) => {
                if !self.database.users.contains_key(&visit.user) {
                    return Err(StatusCode::BadRequest);
                }

                if !self.database.locations.contains_key(&visit.location) {
                    return Err(StatusCode::BadRequest);
                }

                match self.database.visits.entry(visit.id) {
                    Entry::Occupied(_) => return Err(StatusCode::BadRequest),
                    Entry::Vacant(v) => v.insert(visit.clone())
                };

                self.database.visits_by_location.entry(visit.location)
                    .or_insert_with(Default::default)
                    .insert(visit.visited_at, visit.clone());

                self.database.visits_by_user.entry(visit.user)
                    .or_insert_with(Default::default)
                    .insert(visit.visited_at, visit);
            }
        };

        Ok(Bytes::from_static(POST_RESPONSE))
    }
}
