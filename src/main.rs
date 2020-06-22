use http::method::Method;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
const GET: Method = Method::GET;
const POST: Method = Method::POST;
use http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use http::header::CONTENT_TYPE;

struct MyService {}

impl MyService {
    fn new() -> Self {
        Self {}
    }

    async fn handle(&self, _req: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(match *_req.method() {
            GET => {
                let (mut ch, body) = Body::channel();
                tokio::spawn(async move {
                    let _ = ch
                        .send_data(bytes::Bytes::from_static(b"data: 1\n\n"))
                        .await;
                    tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
                    let _ = ch
                        .send_data(bytes::Bytes::from_static(b"data: 2\n\n"))
                        .await;
                    tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
                    let _ = ch
                        .send_data(bytes::Bytes::from_static(b"data: 3\n\n"))
                        .await;
                    let _ = ch
                        .send_data(bytes::Bytes::from_static(b""))
                        .await;
                });
                Response::builder()
                    .header(CONTENT_TYPE, "text/event-stream")
                    .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                    .status(200)
                    .body(body)
                    .unwrap()
            }
            POST => Response::new(Body::from("Hello World\n")),
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
