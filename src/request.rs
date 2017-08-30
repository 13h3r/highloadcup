use data::*;
use serde::de::{Deserializer, Deserialize};

#[derive(Debug)]
pub enum Request {
    Get(GetRequest),
    Post(PostRequest)
}

#[derive(Debug)]
pub enum GetRequest {
    GetEntity(GetEntity),
    GetVisits(UserId, GetVisits),
    GetAverageLocationRating(LocationId, GetAverageLocationRating)
}

#[derive(Debug)]
pub enum PostRequest {
    UpdateEntity(UpdateEntity),
    CreateEntity(CreateEntity)
}

#[derive(Debug)]
pub enum GetEntity {
    User(UserId),
    Location(LocationId),
    Visit(VisitId)
}

#[derive(Default, Debug)]
pub struct GetVisits {
    pub from_date:   Option<Timestamp>,
    pub to_date:     Option<Timestamp>,
    pub country:     Option<String>,
    pub to_distance: Option<u32>
}

#[derive(Default, Debug)]
pub struct GetAverageLocationRating {
    pub from_date: Option<Timestamp>,
    pub to_date:   Option<Timestamp>,
    pub from_age:  Option<Timestamp>,
    pub to_age:    Option<Timestamp>,
    pub gender:    Option<Gender>
}

#[derive(Debug)]
pub enum UpdateEntity {
    User(UserId, UserUpdate),
    Location(LocationId, LocationUpdate),
    Visit(VisitId, VisitUpdate)
}

#[derive(Deserialize, Debug)]
pub struct UserUpdate {
    #[serde(default)]
    pub email:      Optional<String>,
    #[serde(default)]    
    pub first_name: Optional<String>,
    #[serde(default)]    
    pub last_name:  Optional<String>,
    #[serde(default)]    
    pub gender:     Optional<Gender>,
    #[serde(default)]    
    pub birth_date: Optional<Timestamp>
}

#[derive(Deserialize, Debug)]
pub struct LocationUpdate {
    #[serde(default)]    
    pub place:    Optional<String>,
    #[serde(default)]    
    pub country:  Optional<String>,
    #[serde(default)]
    pub city:     Optional<String>,
    #[serde(default)]    
    pub distance: Optional<u32>
}

#[derive(Deserialize, Debug)]
pub struct VisitUpdate {
    #[serde(default)]    
    pub location:   Optional<LocationId>,
    #[serde(default)]    
    pub user:       Optional<UserId>,
    #[serde(default)]    
    pub visited_at: Optional<Timestamp>,
    #[serde(default)]    
    pub mark:       Optional<u8>
}

#[derive(Debug)]
pub enum CreateEntity {
    User(User),
    Location(Location),
    Visit(Visit)
}

// Custom 'Option' type to generate errors when deserializing 'null' value
#[derive(Debug)]
pub enum Optional<T> {
    Something(T),
    Nothing
}

impl<T> Default for Optional<T> {
    #[inline]
    fn default() -> Self {
        Optional::Nothing
    }
}

impl<'de, T> Deserialize<'de> for Optional<T> 
    where T: Deserialize<'de> {
    
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> 
    where
        D: Deserializer<'de>
    {
        let value = T::deserialize(deserializer)?;
        Ok(Optional::Something(value))
    }
}