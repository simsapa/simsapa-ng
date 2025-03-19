use ureq;
use rocket::{get, routes};
use rocket::fs::FileServer;
use rocket::response::content;
use rocket::Shutdown;
use rocket_cors::CorsOptions;

//use std::fs;
use std::path::PathBuf;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::{API_PORT, API_URL};
use crate::html_content::html_page;

#[get("/")]
fn index() -> content::RawHtml<String> {
    content::RawHtml(html_page("", None, None, None))
}

#[get("/shutdown")]
fn shutdown(shutdown: Shutdown) {
    shutdown.notify();
    println!("Webserver shutting down...")
}

#[rocket::main]
#[unsafe(no_mangle)]
pub async extern "C" fn start_webserver() {
    let assets_path = PathBuf::from("./assets/");
    // println!("Serving assets from: {}", assets_path.display());

    let cors = CorsOptions::default().to_cors().expect("Cors options error");

    let config = rocket::Config::figment()
        .merge(("log_level", rocket::config::LogLevel::Off))
        .merge(("address", "127.0.0.1"))
        .merge(("port", API_PORT));

    let _ = rocket::build()
        .configure(config)
        .attach(cors)
        .mount("/assets", FileServer::from(assets_path))
        .mount("/", routes![
            index,
            shutdown
        ])
        .launch().await;
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown_webserver() {
    match ureq::get(format!("{}/shutdown", API_URL)).call() {
        Ok(mut resp) => {
            match resp.body_mut().read_to_string() {
                Ok(body) => { println!("{}", body); }
                Err(_) => { println!("Response error."); }
            }
        },
        Err(ureq::Error::StatusCode(code)) => {
            println!("Error {}", code);
        }
        Err(_) => { println!("Error response from webserver shutdown."); }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown_webserver_tcp() {
    match TcpStream::connect(format!("localhost:{}", API_PORT)) {
        Ok(mut connection) => {
            // Set a timeout of 5 seconds
            if let Err(e) = connection.set_read_timeout(Some(Duration::from_secs(5))) {
                eprintln!("Error setting timeout: {}", e);
            }

            // Construct and send the HTTP GET request
            let request = format!("GET /shutdown HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
            if let Err(e) = connection.write_all(request.as_bytes()) {
                eprintln!("Error sending request: {}", e);
            }

            // Read the response
            let mut response = String::new();
            if let Err(e) = connection.read_to_string(&mut response) {
                eprintln!("Error reading response: {}", e);
            }

            println!("{}", response);
        }
        Err(e) => {
            eprintln!("Error connecting to server: {}", e);
        }
    }
}
