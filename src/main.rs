use http::method::Method;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
const GET: Method = Method::GET;
const POST: Method = Method::POST;

struct MyService{

}

impl MyService {
    fn new() -> Self {
        Self {

        }
    }

    async fn handle(&self, _req: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(match *_req.method() {
            GET => Response::new(Body::from("Hello World\n")),
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
