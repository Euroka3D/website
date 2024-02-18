#![allow(clippy::uninlined_format_args)]

use std::collections::HashSet;

use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use config::Config as CfgLoader;

mod handlers;
mod langs;
use langs::LangGuardRedir;

use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    log_level: log::Level,
    statics_path: (String, String),
    supported_languages: HashSet<String>,
    listen_addr: std::net::SocketAddr,
    ssl_priv:  Option<String>,
    ssl_fullchain: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config: Config = CfgLoader::builder()
        .add_source(config::File::with_name("./settings.toml"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    std::env::set_var("RUST_LOG", config.log_level.as_str());
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .service(
                fs::Files::new(&config.statics_path.0, &config.statics_path.1)
                    .use_last_modified(true),
            )
            .service(
                web::scope("")
                    // First: getting here means that if it's not lang'd, then 404, which implies a
                    // redirect...
                    .wrap(LangGuardRedir {
                        supported_langs: config.supported_languages.clone(),
                    })
                    .service(
                        web::scope("/{lang}")
                            // TODO: make putting this service here redundant.
                            .service(fs::Files::new("/static", "static").use_last_modified(true))
                            .route("", web::get().to(handlers::index))
                            .route("/faq", web::get().to(handlers::faq))
                            .route("/about_page", web::get().to(handlers::about)),
                    ),
            )
    })
    .bind(config.listen_addr)?
    .run()
    .await
}
