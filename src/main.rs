extern crate futures;
extern crate hyper;
extern crate pretty_env_logger;
#[macro_use] extern crate log;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use futures::{Future, Stream};
use hyper::{Get, Post, StatusCode};
use hyper::header::ContentLength;
use hyper::server::{Http, Service, Request, Response};

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
        let (m, uri, _, _, body) = req.deconstruct();
        let path = String::from(uri.path());
        match m { 
            Get => {
                info!("getting at {}, len {}",path, self.data.borrow().len() );
                match self.data.borrow().get(&path) {
                    Some(v) => {
                        info!("Present {}", v);
                        Box::new(futures::future::ok(
                            Response::new()
                                .with_header(ContentLength(v.len() as u64))
                                .with_body(v.clone()),
                        ))
                    } 
                    _ => {
                        info!("Empty");
                        Box::new(futures::future::ok(
                            Response::new().with_status(StatusCode::NotFound),
                        ))
                    }
                }
            }
            Post => {
                info!("Posting");
                let state = self.data.clone();
                Box::new(
                    body.concat2()
                        .and_then(move |c| match String::from_utf8(c.to_vec()) {
                            Ok(s) =>  
                                match state.borrow_mut().insert(path, s.clone()) {
                                    _ => futures::future::ok(Response::new().with_status(StatusCode::Ok)), 
                                }    
                                
                            Err(e) => futures::future::err(
                                hyper::error::Error::Utf8(e.utf8_error()),
                            ),

                        })
                    )
            }
            _ => Box::new(
                futures::future::ok(Response::new().with_status(StatusCode::BadRequest))
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


extern crate hyper;
extern crate tokio_core;
use Echo;
use futures::{Stream, Future};
use futures::future::*;
use hyper::{Client, Method, Request};
use hyper::header::{ContentType, ContentLength};
use hyper::server::Http;
use hyper::client::HttpConnector;
use std::{thread, time};
use std::sync::{Arc, Mutex}; 
use std::result::Result;



struct BlockingClient { 
    core: tokio_core::reactor::Core,
    client: hyper::Client<HttpConnector>,
    port: u32,
}

impl BlockingClient { 
    fn new(port: u32) -> BlockingClient {
        let core = tokio_core::reactor::Core::new().unwrap();
        BlockingClient { 
            client: Client::new(&core.handle()),
            core: core, 
            port: port 
        }
    }

    fn get(&mut self, path :&String) -> Result<String, hyper::StatusCode> {
        let uri : hyper::Uri = self.build_uri(path.clone()); 
        let res = self.core.run(self.client.get(uri.clone())).unwrap(); 
        if res.status() == hyper::StatusCode::Ok {
            Ok(self.core.run(res.body().concat2()
                .and_then(|c| {  
                          let body = String::from_utf8(c.to_vec()).unwrap(); 
                          info!("Built string"); 
                          ok(body)
                })).unwrap())
        } else {
            Err(res.status())
        }
    }

    fn put(&mut self, path :&String, body: &String) -> Result<Option<String>, hyper::StatusCode> {
        let uri : hyper::Uri = self.build_uri(path.clone()); 
        let mut post_req = Request::new(Method::Post, uri.clone());
        post_req.headers_mut().set(ContentType::plaintext());
        post_req.headers_mut().set(ContentLength(body.len() as u64));
        post_req.set_body(body.clone());
        let res_post  = self.core.run(self.client.request(post_req)).unwrap();
        match res_post.status() {
            hyper::StatusCode::Ok => Ok(None),
            _ => Err(res_post.status())
        }
    }

    fn build_uri(&self, path: String) -> hyper::Uri { 
        format!("http://localhost:{}/{}", self.port, path).parse().unwrap() 
    }
}

    #[test]
    fn get_nothing() {
        start_server(1337);
        let mut client = BlockingClient::new(1337);
        let result = client.get(&String::from("foo"));
        assert_eq!(hyper::StatusCode::NotFound, result.unwrap_err());
    }

    #[test]
    fn put_empty_get() { 
        start_server(1338);
        let mut client = BlockingClient::new(1338);
        let body = String::from("123");
        let path = String::from("get_put");

        assert_eq!(None, client.put(&path, &body).unwrap());

        assert_eq!(body, client.get(&path).unwrap()); 
    }
    
    fn start_server(port :u32) { 
        thread::spawn(move || { 
            let addr = format!("127.0.0.1:{}", port).parse().unwrap();

            let server = Http::new().bind(&addr, || Ok(Echo::new())).unwrap();
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(500));
    }

}
