extern crate futures;
extern crate hyper;
extern crate pretty_env_logger;

use std::ops::DerefMut;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use futures::{Future, Stream};
use hyper::{Get, Post, StatusCode};
use hyper::header::ContentLength;
use hyper::server::{Http, Service, Request, Response};


fn add(data: &mut HashMap<String, String>, path: &str, value: String) -> Response {
    match data.insert(path.to_string(), value) { 
        // return the old value
        Some(d) => {
            Response::new()
                .with_header(ContentLength(d.len() as u64))
                .with_body(d.clone())
        }
        _ => Response::new().with_status(StatusCode::Ok),
    }
}

struct Echo {
    data: Rc<RefCell<HashMap<String, String>>>,
}

impl Echo {
    fn new() -> Echo {
        Echo { data: Rc::new(RefCell::new(HashMap::new())) }
    }
}


impl Service for Echo {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let (m, uri, _, headers, body) = req.deconstruct();
        let d = self.data.clone();
        match m { 
            Get => {
                match d.borrow().get(uri.path()) {
                    Some(d) => {
                        Box::new(futures::future::ok(
                            Response::new()
                                .with_header(ContentLength(d.len() as u64))
                                .with_body(d.clone()),
                        ))
                    } 
                    _ => {
                        Box::new(futures::future::ok(
                            Response::new().with_status(StatusCode::NotFound),
                        ))
                    }
                }
            }
            Post => {
                Box::new(
                    body.concat2()
                        .and_then(move |c| match String::from_utf8(c.to_vec()) {
                            Ok(s) => futures::future::ok(s),
                            Err(e) => futures::future::err(
                                hyper::error::Error::Utf8(e.utf8_error()),
                            ),

                        })
                        .and_then(move |c| {
                            add(d.borrow_mut().deref_mut(), uri.path(), c.clone());
                            futures::future::ok((Response::new().with_body(c)))
                        }),
                )
            }
            _ => Box::new(
                futures::future::ok(Response::new().with_status(StatusCode::BadRequest)).boxed(),
            ),
        }
    }
}

fn main() {
    pretty_env_logger::init().unwrap();
    let addr = "127.0.0.1:1337".parse().unwrap();

    let server = Http::new().bind(&addr, || Ok(Echo::new())).unwrap();
    println!(
        "Listeningon http://{} with 1 thread.",
        server.local_addr().unwrap()
    );

    server.run().unwrap();
}


#[cfg(test)]
mod test {

    #[test]
    fn put() {
        assert!(false);
    }
}
