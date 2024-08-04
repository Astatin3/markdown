use core::time::Duration;
use std::{env, process};
use std::ffi::OsStr;
use hotwatch::{Hotwatch};
use rocket::{State, Shutdown};
use rocket::fs::NamedFile;
use rocket::response::stream::{Event as SSEvent, EventStream};
use rocket::serde::Serialize;
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket::tokio::select;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use time::Error;

#[macro_use] extern crate rocket;

#[derive(Debug, Clone, Serialize)]
#[serde(crate = "rocket::serde")]
struct FileChange {
    content: String,
}


#[get("/")]
async fn index() -> NamedFile {
    NamedFile::open("index.html").await.unwrap()
}

#[get("/content")]
async fn content(file_content: &State<Arc<Mutex<String>>>) -> String {
    file_content.lock().unwrap().clone()
}

#[get("/events")]
async fn events(queue: &State<Sender<FileChange>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(_) => break,
                },
                _ = &mut end => break,
            };
            yield SSEvent::json(&msg);
        }
    }
}

fn get_filepath() -> String {
    let args = env::args().collect::<Vec<String>>();
    return String::from(args[1].as_str());
}

#[launch]
fn rocket() -> _ {

    let file_content = Arc::new(Mutex::new(std::fs::read_to_string(get_filepath()).unwrap()));
    let file_content_clone = Arc::clone(&file_content);

    let (tx, _) = channel::<FileChange>(1024);
    let tx_clone = tx.clone();



    std::thread::spawn(move || {
        let mut watch = Hotwatch::new().unwrap();

        watch.watch(get_filepath(), move |_| {

            let content = std::fs::read_to_string(get_filepath()).unwrap();

            *file_content_clone.lock().unwrap() = content.clone();
            let _ = tx_clone.send(FileChange { content });

            std::thread::sleep(Duration::from_secs(1));
        }).unwrap();

        loop { // IDK how to sleep forever
            std::thread::sleep(Duration::from_secs(9223372036854775807));
        }
    });

    rocket::build()
        .manage(tx)
        .manage(file_content)
        .mount("/", routes![index, content, events])
}