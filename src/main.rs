use bytes::Buf;
use http::method::Method;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::io::BufRead;
use std::net::SocketAddr;
use tokio::sync::broadcast::error::RecvError;
const GET: Method = Method::GET;
const POST: Method = Method::POST;
use http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use http::header::CONTENT_TYPE;

use dashmap::DashMap;
use tokio::sync::broadcast::{channel, Sender};

struct Loc {
    sender: Sender<bytes::Bytes>,
}

struct MyService {
    locs: DashMap<String, Loc>,
}

impl MyService {
    fn new() -> Self {
        Self {
            locs: DashMap::new(),
        }
    }

    async fn handle(&self, _req: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(match *_req.method() {
            GET => {
                let mut rx = {
                    let path = _req.uri().path();
                    let loc_lock = self.locs.entry(path.to_string()).or_insert_with(|| {
                        let (sender, _rx) = channel(2);
                        Loc { sender }
                    });
                    loc_lock.sender.subscribe()
                };
                let (mut ch, body) = Body::channel();
                tokio::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(msg) => {
                                if let Err(_) = ch.send_data(msg).await {
                                    break;
                                }
                            }
                            Err(RecvError::Closed) => break, /* should not happen */
                            Err(RecvError::Lagged(_n)) => continue,
                        }
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
                let path = _req.uri().path();
                match self.locs.get(path) {
                    None => Response::builder()
                        .status(404)
                        .body(Body::from(
                            "No ongoing GET queries are listening for POST data on this URL.\n",
                        ))
                        .unwrap(),
                    Some(loc_lock) => {
                        let sender = loc_lock.sender.clone();
                        drop(loc_lock);
                        let body = _req.into_body();
                        if let Ok(body) = hyper::body::aggregate(body).await {
                            // Convert lines of `msg` to text/event-stream format
                            let mut buf = bytes::BytesMut::with_capacity(body.remaining() + 12);
                            let mut sbuf = String::with_capacity(body.remaining().min(128));
                            let mut buf_reader = body.reader();

                            loop {
                                sbuf.clear();
                                match buf_reader.read_line(&mut sbuf) {
                                    Ok(n) => {
                                        if n == 0 {
                                            break;
                                        }
                                        buf.extend_from_slice(b"data: ");
                                        buf.extend_from_slice(sbuf.as_bytes());
                                        if !sbuf.ends_with('\n') {
                                            buf.extend_from_slice(b"\n");
                                        }
                                    }
                                    Err(_) => {
                                        return Ok(Response::builder().status(400).body(Body::from("Binary data is not supported by HTTP's text/event-stream\n")).unwrap());
                                    }
                                }
                            }
                            buf.extend_from_slice(b"\n");

                            if let Err(_) = sender.send(buf.freeze()) {
                                Response::builder()
                                    .status(404)
                                    .body(Body::from(
                                        "There was a receiver for this URL, but it is gone now.\n",
                                    ))
                                    .unwrap()
                            } else {
                                Response::builder().status(204).body(Body::empty()).unwrap()
                            }
                        } else {
                            Response::builder()
                                .status(500)
                                .body(Body::from("Failed reading input data from POST request.\n"))
                                .unwrap()
                        }
                    }
                }
            }
            _ => Response::builder()
                .status(400)
                .body(Body::from("Method should be GET or POST\n"))
                .unwrap(),
        })
    }
}

#[tokio::main(flavor = "current_thread")]
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
