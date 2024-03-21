use std::convert::Infallible;
use std::error::Error;

use futures::future::ok;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use hyper::{Body, Request, Response, Server, service::{make_service_fn, service_fn}};
use rss::Channel;
use serde::Serialize;

static STATE: Lazy<Mutex<Option<Channel>>> = Lazy::new(|| {
    Mutex::new(None)
});

#[derive(Serialize)]
struct ChannelResponse {
  description: String
}

async fn xkcd_feed() -> Result<Channel, Box<dyn Error>> {
    let content = reqwest::get("https://xkcd.com/rss.xml")
        .await?
        .bytes()
        .await?;

    let channel = Channel::read_from(&content[..])?;

    Ok(channel)
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();

    if path == "/feed" {
        let channel =  STATE.lock().await.clone().unwrap_or_default();       
        let response = ChannelResponse { description: channel.description };
        let body = serde_json::to_string(&response).unwrap_or("{}".to_string());

        return ok(Response::new(Body::from(body))).await;
    }

    Ok(Response::new(Body::from("Not found")))
}

#[tokio::main]
async fn main() {    
    tokio::spawn(async move {
        let mut lock = STATE.lock().await;
        *lock = xkcd_feed().await.ok()
    });

    let addr = ([127, 0, 0, 1], 8080).into();

    let make_service = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let server = Server::bind(&addr).serve(make_service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
