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


#[macro_use]
extern crate lazy_static;
#[cfg(test)]
mod test {


extern crate hyper;
extern crate tokio_core;
use Echo;
use futures::Future;
use hyper::{Client, Method, Request};
use hyper::header::{ContentType, ContentLength};
use hyper::server::Http;
use std::{thread, time};
use std::sync::{Arc, Mutex}; 

lazy_static! { 
    static ref SERVER_STATE:Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

const URI_BASE: &str= "http://localhost:1337"; 

    #[test]
    fn get_nothing() {
        start_server();
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let client = Client::new(&core.handle());
        let uri : hyper::Uri = format!("{}/foo", URI_BASE).parse().unwrap();
        let res = client.get(uri.clone()).map(|res| {
            res.status() 
        });
        let f = core.run(res);
        assert_eq!(f.unwrap(), hyper::NotFound);
    }

    #[test]
    fn put_empty_get() { 
        // put 
        let mut core = tokio_core::reactor::Core::new().unwrap();
        let client = Client::new(&core.handle());
        let uri : hyper::Uri = format!("{}/get_put", URI_BASE).parse().unwrap();
        let mut post_req = Request::new(Method::Post, uri.clone());
        let body = "123";
        post_req.headers_mut().set(ContentType::plaintext());
        post_req.headers_mut().set(ContentLength(body.len() as u64));
        post_req.set_body("123");
        let res_post  = client.request(post_req).map(|res| { 
            res.status()
        });
        let f = core.run(res_post);
        assert_eq!(f.unwrap(), hyper::Ok);

        // now do the get
        let mut res = client.get(uri.clone()).map(|res| {
            (res.status(), 
            res.body())
        });
        let f = core.run(res);
        let (code, b) = f.unwrap();  
        assert_eq!(hyper::Ok, code);
        // TODO: STart here with pulling in body
        //assert_eq!("123", b);
    }
    
    fn start_server() { 
        let local_state = SERVER_STATE.clone();
        let mut server_init = local_state.lock().unwrap();
        if *server_init { 
            return;
        }
        *server_init = true;
        thread::spawn(move || { 
            let addr = "127.0.0.1:1337".parse().unwrap();

            let server = Http::new().bind(&addr, || Ok(Echo::new())).unwrap();
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(2000));
    }

}
