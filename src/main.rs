#![allow(clippy::uninlined_format_args)]
use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};

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

impl Lang {
    fn from_accept_lang_header(langs_str: &str) -> Result<Lang, ()> {
        let mut best_lang = None;
        let mut highest_qual = 0.0;

        for entry in langs_str.split(',') {
            // separate the language and the priority
            let mut parts = entry.split(';');
            let lang = parts.next().unwrap_or_default().trim();
            let lang = Lang::try_from(lang).unwrap_or_default();
            let Some(qual_part) = parts.next() else {
                return Ok(lang);
            };
            let stripped_qual: &str = qual_part
                .strip_prefix("q=")
                .expect("todo: trigger a malformed header error");
            let quality: f32 = stripped_qual
                .parse::<f32>()
                .expect("todo: trigger a malformed header error");
            if quality == 1.0 {
                return Ok(lang);
            }
            if quality == 0.0 {
                continue;
            }
            if best_lang.is_none() {
                best_lang = Some(lang);
                highest_qual = quality;
                continue;
            };
            if quality > highest_qual {
                best_lang = Some(lang);
                highest_qual = quality;
            }
        }
        best_lang.ok_or(())
    }
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
