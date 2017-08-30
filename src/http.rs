use std::sync::{RwLock, Arc};
use std::ops::Deref;

use futures::future::Future;
use futures::stream::Stream;

use hyper::server::Service;
use hyper::{self, Method, Response as HttpResponse, Request as HttpRequest};
use hyper::header::{Headers, ContentLength};

use api::Api;
use router;

pub struct TravelsServer {
    pub api: Arc<RwLock<Api>>,
}

#[inline]
fn read_to_end<S, I>(stream: S) -> impl Future<Item = Vec<u8>, Error = hyper::Error>
where
    S: Stream<Item = I, Error = hyper::Error>,
    I: Deref<Target = [u8]>,
{
    type Buffer = Result<Vec<u8>, hyper::Error>;
    stream.fold(Vec::with_capacity(512), |mut buffer, chunk| -> Buffer {
        buffer.extend_from_slice(&chunk);
        Ok(buffer)
    })
}

impl Service for TravelsServer {
    type Request = HttpRequest;
    type Response = HttpResponse;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    #[inline]
    fn call(&self, request: Self::Request) -> Self::Future {
        let (method, uri, _http_version, _headers, body) = request.deconstruct();
        let is_post = method == Method::Post;
        let read_body = read_to_end(body);

        let api = self.api.clone();
        let http_response = read_body.map(move |body| {
            use request::Request;
            let result = router::route(method, uri, &body)
                .and_then(|request| match request {
                    Request::Get(request) => {
                        let lock = api.read().expect("Failed to lock (read)");
                        lock.do_get(request)
                    }
                    Request::Post(request) => {
                        let mut lock = api.write().expect("Failed to lock (write)");
                        lock.do_post(request)
                    }
            });

            match result {
                Ok(response) => {
                    let headers = {
                        let mut headers = Headers::with_capacity(3);
                        headers.set(ContentLength(response.len() as u64));
                        // raw headers to avoid allocation
                        headers.set_raw("Content-Type", "application/json");
                        if is_post {
                            headers.set_raw("Connection", "close");
                        } else {
                            headers.set_raw("Connection", "keep-alive");
                        }
                        headers
                    };

                    HttpResponse::new()
                        .with_headers(headers)
                        .with_body(response)
                }
                Err(code) => {
                    let headers = {
                        let mut headers = Headers::with_capacity(2);
                        headers.set_raw("Content-Type", "json");
                        if is_post {
                            headers.set_raw("Connection", "close");
                        } else {
                            headers.set_raw("Connection", "keep-alive");
                        }
                        headers
                    };

                    HttpResponse::new()
                        .with_headers(headers)
                        .with_status(code)
                }
            }
        });

        Box::new(http_response)
    }
}
