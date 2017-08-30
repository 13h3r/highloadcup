use hyper::{StatusCode, Uri, Method};

use data::{LocationId, UserId, VisitId};
use request::{self, GetEntity, CreateEntity, UpdateEntity, Request as ApiRequest, GetRequest, PostRequest};

#[inline]
pub fn route(method: Method, uri: Uri, body: &[u8]) -> Result<ApiRequest, StatusCode> {
    match method {
        Method::Get => route_get_request(uri).map(ApiRequest::Get),
        Method::Post => route_post_request(uri, body).map(ApiRequest::Post),
        _ => Err(StatusCode::BadRequest),
    }
}

#[inline]
fn route_get_request(uri: Uri) -> Result<GetRequest, StatusCode> {
    let path = uri.path();
    let id: u32 = path.split('/')
        .nth(2)
        .ok_or(StatusCode::BadRequest)?
        .parse()
        .map_err(|_| StatusCode::NotFound)?;

    let request = if path.ends_with("/avg") {
        let parameters = {
            match uri.query() {
                Some(query) => parse_alr_parameters(query)?,
                None => Default::default()
            }
        };
        GetRequest::GetAverageLocationRating(LocationId(id), parameters)
    } else if path.ends_with("/visits") {
        let parameters = {
            match uri.query() {
                Some(query) => parse_visits_parameters(query)?,
                None => Default::default()
            }
        };
        GetRequest::GetVisits(UserId(id), parameters)
    } else {
        let request = match path.split('/').nth(1).ok_or(StatusCode::NotFound)? {
            "users" => GetEntity::User(UserId(id)),
            "locations" => GetEntity::Location(LocationId(id)),
            "visits" => GetEntity::Visit(VisitId(id)),
            _ => return Err(StatusCode::BadRequest),
        };

        GetRequest::GetEntity(request)
    };

    Ok(request)
}

#[inline]
fn parse_visits_parameters(query: &str) -> Result<request::GetVisits, StatusCode> {
    let mut result = request::GetVisits::default();

    for pair in query.split('&') {
        let mut iter = pair.split('=');
        let name  = iter.next().ok_or(StatusCode::BadRequest)?;
        let value = iter.next().ok_or(StatusCode::BadRequest)?;

        match name {
            "fromDate" => {
                let from_date = value.parse()
                    .map_err(|_| StatusCode::BadRequest)?;
                result.from_date = Some(from_date);
            },
            "toDate" => {
                let to_date = value.parse()
                    .map_err(|_| StatusCode::BadRequest)?;
                result.to_date = Some(to_date);
            },
            "country" => {
                use percent_encoding;
                let country = percent_encoding::percent_decode(value.as_bytes())
                    .decode_utf8()
                    .map_err(|_| StatusCode::BadRequest)?
                    // hack for 'application/x-www-form-urlencoded' percent encoding
                    .replace('+', " ");

                result.country = Some(country.into());
            },
            "toDistance" => {
                let to_distance = value.parse()
                    .map_err(|_| StatusCode::BadRequest)?;
                result.to_distance = Some(to_distance);
            },
            _ => return Err(StatusCode::BadRequest)
        }
    }

    Ok(result)
}

#[inline]
fn parse_alr_parameters(query: &str) -> Result<request::GetAverageLocationRating, StatusCode> {
    use data::Gender;

    let mut result = request::GetAverageLocationRating::default();
    for pair in query.split('&') {
        let mut iter = pair.split('=');
        let name  = iter.next().ok_or(StatusCode::BadRequest)?;
        let value = iter.next().ok_or(StatusCode::BadRequest)?;

        match name {
            "fromDate" => result.from_date = Some(value.parse()
                .map_err(|_| StatusCode::BadRequest)?),
            "toDate" => result.to_date = Some(value.parse()
                .map_err(|_| StatusCode::BadRequest)?),
            "fromAge" => result.from_age = Some(value.parse()
                .map_err(|_| StatusCode::BadRequest)?),
            "toAge" => result.to_age = Some(value.parse()
                .map_err(|_| StatusCode::BadRequest)?),
            "gender" => {
                match value {
                    "m" => result.gender = Some(Gender::Male),
                    "f" => result.gender = Some(Gender::Female),
                    _ => return Err(StatusCode::BadRequest),
                }
            }
            _ => return Err(StatusCode::BadRequest),
        };
    }

    Ok(result)
}

#[inline]
fn route_post_request(uri: Uri, body: &[u8]) -> Result<PostRequest, StatusCode> {
    use serde_json;

    let (entity, id) = {
        let path = uri.path();
        let mut iter = path.split('/').skip(1);
        let entity = iter.next().ok_or(StatusCode::NotFound)?;
        let id = iter.next().ok_or(StatusCode::NotFound)?;
        (entity, id)
    };

    let request = if id == "new" {
        let request = match entity {
            "users" => {
                let user = serde_json::from_slice(body)
                    .map_err(|_| StatusCode::BadRequest)?;
                CreateEntity::User(user)
            }
            "locations" => {
                let location = serde_json::from_slice(body)
                    .map_err(|_| StatusCode::BadRequest)?;
                CreateEntity::Location(location)
            }
            "visits" => {
                let visit = serde_json::from_slice(body)
                    .map_err(|_| StatusCode::BadRequest)?;
                CreateEntity::Visit(visit)
            }
            _ => return Err(StatusCode::BadRequest),
        };

        PostRequest::CreateEntity(request)
    } else {
        let id: u32 = id.parse().map_err(|_| StatusCode::NotFound)?;
        let request = match entity {
            "users" => {
                let user_update = serde_json::from_slice(body)
                    .map_err(|_| StatusCode::BadRequest)?;
                UpdateEntity::User(UserId(id), user_update)
            }
            "locations" => {
                let location_update = serde_json::from_slice(body)
                    .map_err(|_| StatusCode::BadRequest)?;
                UpdateEntity::Location(LocationId(id), location_update)
            }
            "visits" => {
                let visit_update = serde_json::from_slice(body)
                    .map_err(|_| StatusCode::BadRequest)?;
                UpdateEntity::Visit(VisitId(id), visit_update)
            }
            _ => return Err(StatusCode::BadRequest),
        };

        PostRequest::UpdateEntity(request)
    };

    Ok(request)
}