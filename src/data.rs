use std::fmt;

use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde::de::{Visitor, Error};

#[derive(Hash, Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(pub u32);

#[derive(Hash, Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationId(pub u32);

#[derive(Hash, Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisitId(pub u32);
pub type Timestamp = i64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gender {
    Male,
    Female,
}

impl Serialize for Gender {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let identifier = match self {
            &Gender::Female => "f",
            &Gender::Male => "m",
        };

        serializer.serialize_str(identifier)
    }
}

struct GenderVisitor;
impl<'de> Visitor<'de> for GenderVisitor {
    type Value = Gender;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an 'f' or 'm'")
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match v {
            "m" => Ok(Gender::Male),
            "f" => Ok(Gender::Female),
            _ => Err(E::custom("Incorrect gender identifier")),
        }
    }
}

impl<'de> Deserialize<'de> for Gender {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Gender, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(GenderVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id:         UserId,
    pub email:      String,
    pub first_name: String,
    pub last_name:  String,
    pub gender:     Gender,
    pub birth_date: Timestamp, 
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub id:       LocationId,
    pub place:    String,
    pub country:  String,
    pub city:     String,
    pub distance: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Visit {
    pub id:         VisitId,
    pub location:   LocationId,       
    pub user:       UserId,       
    pub visited_at: Timestamp, 
    pub mark:       u8,        // in range 0..5
}


#[cfg(test)]
mod tests {
    use serde_json;
    use super::*;

    #[test]
    fn serialize_gender() {
        let s = Gender::Male;
        let serialized = serde_json::to_string(&s).unwrap();
        assert_eq!(serialized, "\"m\"");
    }

    #[test]
    fn deserialize_gender() {
        let gender: Gender = serde_json::from_str("\"f\"").unwrap();
        assert_eq!(gender, Gender::Female);
    }

    #[test]
    fn serialize_user() {
        let user = User {
            id: UserId(1),
            email: "robosen@icloud.com".to_string(),
            first_name: "Данила".to_string(),
            last_name: "Стамленский".to_string(),
            gender: Gender::Male,
            birth_date: 345081600
        };

        let user = serde_json::to_string(&user).unwrap();
        let expected_user = 
            r#"{"id":1,"email":"robosen@icloud.com","first_name":"Данила","last_name":"Стамленский","gender":"m","birth_date":345081600}"#;
        
        assert_eq!(user, expected_user);
    }

    #[test]
    fn deserialize_user() {
        let data = r#"[
            {
                "id": 1,
                "email": "robosen@icloud.com",
                "first_name": "Данила",
                "last_name": "Стамленский",
                "gender": "m",
                "birth_date": 345081600
            }, 
            {
                "id": 2,
                "email": "tameerne@yandex.ru",
                "first_name": "Аня",
                "last_name": "Шишкина",
                "gender": "f",
                "birth_date": -1571356800
            }
        ]"#;

        let users: Vec<User> = serde_json::from_str(data).unwrap();
        assert_eq!(users.len(), 2);
        
        assert_eq!(users[0], User {
            id: UserId(1),
            email: "robosen@icloud.com".to_string(),
            first_name: "Данила".to_string(),
            last_name: "Стамленский".to_string(),
            gender: Gender::Male,
            birth_date: 345081600
        });

        assert_eq!(users[1], User {
            id: UserId(2),
            email: "tameerne@yandex.ru".to_string(),
            first_name: "Аня".to_string(),
            last_name: "Шишкина".to_string(),
            gender: Gender::Female,
            birth_date: -1571356800
        });
    }
}