#![allow(unused)]

use http::method::Method;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
const GET: Method = Method::GET;
const POST: Method = Method::POST;
use http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use http::header::CONTENT_TYPE;

use dashmap::DashMap as HashMap;
use tokio::sync::broadcast::{Sender,Receiver,channel};


struct Loc {
    s: Sender<String>,
    //r: Receiver<String>,
}

struct MyService {
    locs: HashMap<String, Loc>,
}

impl MyService {
    fn new() -> Self {
        Self {
            locs: HashMap::new(),
        }
    }

    async fn handle(&self, _req: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(match *_req.method() {
            GET => {
                let mut r = {
                    let p = _req.uri().path();
                    let l = self.locs.entry(p.to_string()).or_insert_with(||{
                        let (s,_r) = channel(2);
                        Loc { s }
                    });
                    l.s.subscribe()
                };
                let (mut ch, body) = Body::channel();
                tokio::spawn(async move {
                    while let Ok(msg) = r.recv().await {
                        for l in msg.lines().map(|x|x.to_string()) {
                            let _ = ch
                            .send_data(bytes::Bytes::from_static(b"data: "))
                            .await;

                        let _ = ch
                        .send_data(bytes::Bytes::from(l))
                        .await;

                            let _ = ch
                            .send_data(bytes::Bytes::from_static(b"\n"))
                            .await;
                        }

                        let _ = ch
                        .send_data(bytes::Bytes::from_static(b"\n"))
                        .await;

                    }
                });
                Response::builder()
                    .header(CONTENT_TYPE, "text/event-stream")
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .status(200)
                    .body(body)
                    .unwrap()
            }
            POST => {
                let p = _req.uri().path();
                match self.locs.get(p) {
                    None => {
                        Response::builder().status(404).body(Body::empty()).unwrap()
                    }
                    Some(l) => {
                        let s = l.s.clone();
                        drop(l);
                        let b = _req.into_body();
                        tokio::spawn(async move {
                            use bytes::Buf;
                            if let Ok(msg) = hyper::body::aggregate(b).await {
                                if let Ok(msg) = String::from_utf8(msg.bytes().to_vec()) {
                                    s.send(msg);
                                }
                            }
                        });
                        Response::builder().status(204).body(Body::empty()).unwrap()
                    }
                }
                
            },
            _ => Response::builder()
                .status(400)
                .body(Body::from("Method should be GET or POST\n"))
                .unwrap(),
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = std::env::args()
        .into_iter()
        .nth(1)
        .ok_or(anyhow::anyhow!("Specify listen address"))?;
    let addr: SocketAddr = addr.parse()?;
    let serv = std::sync::Arc::new(MyService::new());
    let make_service = make_service_fn(move |_conn| {
        let serv = serv.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |r| {
                let serv = serv.clone();
                async move { serv.handle(r).await }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_service);
    server.await?;
    Ok(())
}
