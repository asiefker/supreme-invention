extern crate env_logger;
#[macro_use]
extern crate log;
extern crate futures;
extern crate hyper;

use hyper::{Post, StatusCode};
use hyper::header::ContentLength;
use hyper::server::{Http, Service, Request, Response};
use futures::{future, Future, Stream};

#[derive(Clone)]
struct Echo;

impl Service for Echo {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = futures::BoxFuture<Response, hyper::Error>;

    fn call(&self, req: Request) -> Self::Future {
        let (method, uri, _version, headers, body) = req.deconstruct();
        match (method, uri.path()) {
            (Post, "/echo") => {
                let mut res = Response::new();
                let vec;
                if let Some(len) = headers.get::<ContentLength>() {
                    vec = Vec::with_capacity(**len as usize);
                    res.headers_mut().set(len.clone());
                } else {
                    vec = vec![];
                }
                body.fold(vec, |mut acc, chunk| {
                    acc.extend_from_slice(chunk.as_ref());
                    Ok::<_, hyper::Error>(acc)
                }).and_then(move |value| {
                        debug!("value: {:?}", &value);
                        Ok(res.with_body(value))
                    })
                    .boxed()
            }
            _ => future::ok(Response::new().with_status(StatusCode::NotFound)).boxed(),
        }
    }
}

fn main() {
    env_logger::init().unwrap();
    let addr = "127.0.0.1:1337".parse().unwrap();
    let server = Http::new().bind(&addr, move || Ok(Echo)).unwrap();
    println!("Listening on http://{}", server.local_addr().unwrap());
    server.run().unwrap();
}
