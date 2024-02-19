#![allow(clippy::uninlined_format_args)]

use std::{collections::HashSet, fs::File, io::BufReader};

use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use config::Config as CfgLoader;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    version::TLS13,
    ServerConfig,
};
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::Deserialize;

mod handlers;
mod langs;
use langs::LangGuardRedir;

#[derive(Deserialize)]
struct Config {
    log_level: log::Level,
    statics_path: (String, String),
    supported_languages: HashSet<String>,
    listen_addr: std::net::SocketAddr,
    cert_pem: Option<String>,
    key_pem: Option<String>,
}

fn load_rustls_config(cfg: &Config) -> rustls::ServerConfig {
    let (Some(cert_name), Some(keys_name)) = (&cfg.cert_pem, &cfg.key_pem) else {
        panic!("cert-files not found");
    };
    let tls_cfg = ServerConfig::builder_with_protocol_versions(&[&TLS13]).with_no_client_auth();

    let cert_file = &mut BufReader::new(File::open(cert_name).unwrap());
    let key_file = &mut BufReader::new(File::open(keys_name).unwrap());

    let cert_chain: Vec<CertificateDer> = certs(cert_file).map(Result::unwrap).collect();

    let mut keys: Vec<PrivateKeyDer> = pkcs8_private_keys(key_file)
        .take(1)
        .map(Result::unwrap)
        .map(Into::into)
        .collect();

    let ssl_config = tls_cfg
        .with_single_cert(
            cert_chain,
            keys.pop().expect("Could not locate PKCS 8 private keys."),
        )
        .unwrap();

    ssl_config
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

    let tls_config = load_rustls_config(&config);
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
    .bind_rustls_0_22(config.listen_addr, tls_config)?
    .run()
    .await
}
