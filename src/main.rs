#![allow(clippy::uninlined_format_args)]
use actix_files as fs;
use actix_web::{guard, middleware, web, App, HttpServer};

mod handlers;
mod langs;
use langs::LanguageConcierge;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .service(fs::Files::new("/static", "static").use_last_modified(true)) // Serve static files
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/{lang}")
                    .guard(guard::fn_guard(|ctx| {
                        ctx.head()
                            .uri
                            .path()
                            .trim_start_matches('/')
                            .split('/')
                            .next()
                            .map_or(false, |rp| ["en", "fr", "de"].contains(&rp))
                    }))
                    .wrap(LanguageConcierge)
                    .route("", web::get().to(handlers::index))
                    .route("/faq", web::get().to(handlers::faq))
                    .route("/about_page", web::get().to(handlers::about)),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
