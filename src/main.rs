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

struct HashStorage {
    data: HashMap<String, String>,
}

impl HashStorage {
    fn new() -> HashStorage {
        HashStorage { data: HashMap::new() }
    }
}

trait Storage {
    fn put(&mut self, key: String, value: String) -> Option<String>;
    fn get(&self, key: &String) -> Option<String>;
    fn len(&self) -> usize;
}

impl Storage for HashStorage {
    fn get(&self, key: &String) -> Option<String> {
        self.data.get(key).map(|s| {s.clone()})   
    }

    fn put(&mut self, key: String, value: String) -> Option<String> {
        self.data.insert(key, value)
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

struct MyServer<'a> {
    storage: Rc<RefCell<Storage +'a>>,
}

impl <'a> MyServer<'a> {
    pub fn new<T>(storage : T) -> MyServer<'a> where T: Storage + 'a {
        MyServer{storage: Rc::new(RefCell::new(storage)) }
    }
}

impl Service for MyServer<'static> {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response, Error = hyper::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let (m, uri, _, _, body) = req.deconstruct();
        let path = String::from(uri.path());
        match m { 
            Get => {
                info!("getting at {}, len {}",path, self.storage.borrow().len() );
                match self.storage.borrow().get(&path) {
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
                let storage = self.storage.clone();
                Box::new(
                    body.concat2()
                        .and_then(move |c| match String::from_utf8(c.to_vec()) {
                            Ok(s) =>  
                                match storage.borrow_mut().put(path, s.clone()) {
                                    Some(b) => { 
                                        futures::future::ok(Response::new()
                                                                .with_status(StatusCode::Ok)
                                                                .with_body(b))}, 
                                    None => futures::future::ok(Response::new().with_status(StatusCode::Ok)), 
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

    let server = Http::new().bind(&addr, || Ok(MyServer::new(HashStorage::new()))).unwrap();
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
use {MyServer,HashStorage};
use futures::{Stream, Future};
use futures::future::*;
use hyper::{Client, Method, Request};
use hyper::header::{ContentType, ContentLength};
use hyper::server::Http;
use hyper::client::HttpConnector;
use std::{thread, time};
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
        match res.status() { 
            hyper::StatusCode::Ok => { 
                let body = self.build_body(res); 
                match body.as_ref() { 
                    "" => Err(hyper::StatusCode::NotFound),
                     _ => Ok(body)
                }
            },
            _ => Err(res.status()),
        } 
    }

    fn build_body(&mut self, res :hyper::Response) -> String { 
        self.core.run(res.body().concat2()
            .and_then(|c| {  
                        let body = String::from_utf8(c.to_vec()).unwrap(); 
                        info!("Built string"); 
                        ok(body)
            })).unwrap()
    }

    fn put(&mut self, path :&String, body: &String) -> Result<Option<String>, hyper::StatusCode> {
        let uri : hyper::Uri = self.build_uri(path.clone()); 
        let mut post_req = Request::new(Method::Post, uri.clone());
        post_req.headers_mut().set(ContentType::plaintext());
        post_req.headers_mut().set(ContentLength(body.len() as u64));
        post_req.set_body(body.clone());
        let res_post  = self.core.run(self.client.request(post_req)).unwrap();
        match res_post.status() {
            hyper::StatusCode::Ok => { 
                let body = self.build_body(res_post);
                match body.as_ref() { 
                    "" => Ok(None),
                    _ => Ok(Some(body))    
                }
            },
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
    
  #[test]
    fn put_put_get() { 
        start_server(1339);
        let mut client = BlockingClient::new(1339);
        let body = String::from("a body");
        let body2 = String::from("A different body with a pithy saying");
        let path = String::from("put_put");

        assert_eq!(None, client.put(&path, &body).unwrap());
        assert_eq!(Some(body), client.put(&path, &body2).unwrap());

        assert_eq!(body2, client.get(&path).unwrap()); 
    }

    fn start_server(port :u32) { 
        thread::spawn(move || { 
            let addr = format!("127.0.0.1:{}", port).parse().unwrap();

            let server = Http::new().bind(&addr, || Ok(MyServer::new(HashStorage::new()))).unwrap();
            server.run().unwrap();
        });
        thread::sleep(time::Duration::from_millis(500));
    }

}
