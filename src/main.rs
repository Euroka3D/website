#![allow(clippy::uninlined_format_args)]
use actix_files as fs;
use actix_web::{http::header::LanguageTag, middleware, web, App, HttpServer};

mod handlers;
mod langs;
use langs::LanguageConcierge;

#[derive(Debug, Hash, Eq, PartialEq)]
enum Lang {
    En,
    Fr,
    // German
    De,
    He,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .service(fs::Files::new("/static", "static").use_last_modified(true)) // Serve static files
            .wrap(middleware::NormalizePath::trim())
            .wrap(LanguageConcierge)
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/{lang}")
                    .route("", web::get().to(handlers::index))
                    .route("/", web::get().to(handlers::index))
                    .route("/faq", web::get().to(handlers::faq))
                    .route("/about", web::get().to(handlers::about)),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
