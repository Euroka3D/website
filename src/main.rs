#![allow(clippy::uninlined_format_args)]

use std::collections::HashSet;

use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};

mod handlers;
mod langs;
use langs::LangGuardRedir;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .service(fs::Files::new("/static", "static").use_last_modified(true))
            // First: getting here means that if it's not lang'd, then 404, which implies a
            // redirect...
            .wrap(LangGuardRedir {
                supported_langs: HashSet::from([
                    "en".to_string(),
                    "fr".to_string(),
                    "de".to_string(),
                ])
                .clone(),
            })
            .service(
                web::scope("/{lang}")
                    // TODO: make putting this service here redundant.
                    .service(fs::Files::new("/static", "static").use_last_modified(true))
                    .route("", web::get().to(handlers::index))
                    .route("/faq", web::get().to(handlers::faq))
                    .route("/about_page", web::get().to(handlers::about)),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
