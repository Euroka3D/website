use actix_files as fs;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;
use fluent::{FluentBundle, FluentResource};
use std::borrow::Borrow;
use unic_langid::LanguageIdentifier; // Make sure you include the Template trait

// Define your Askama template
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    our_services: String,
    services: Vec<Service>,
    learn_more: String,
}
fn load_fluent_bundles(lang: &LanguageIdentifier) -> FluentBundle<FluentResource> {
    let ftl_path = format!("locales/{}.ftl", lang);
    let ftl_string = std::fs::read_to_string(&ftl_path)
        .unwrap_or_else(|_| panic!("Failed to load FTL file: {}", &ftl_path));
    let resource = FluentResource::try_new(ftl_string)
        .unwrap_or_else(|_| panic!("Failed to parse the FTL file: {}", ftl_path));
    let mut bundle = FluentBundle::new(vec![lang.clone()]);
    bundle.add_resource(resource).unwrap_or_else(|_| panic!(
        "Failed to add FTL resource to the bundle: {}",
        lang
    ));

    bundle
}

// For simplicity, define a struct to represent your services
struct Service {
    name: String,
    description: String,
    link: String,
}

async fn en_redirect() -> impl Responder {
    HttpResponse::Found()
        .append_header(("Location", "/en/"))
        .finish()
}

async fn index(lang: web::Path<String>) -> impl Responder {
    log::info!("path: {}", lang);
    // TODO: custom extractor
    let lang_code = lang.borrow();

    // Supported languages
    let supported_languages = ["en", "fr", "de"];

    if !supported_languages.contains(&lang_code.as_str()) {
        return HttpResponse::Ok().body(format!("'{}' not a supported language.", lang_code));
    }
    //
    // Assume English as the default language
    let lang = lang_code.parse().unwrap(); //_or_else(|_| "en".parse().expect("parsing_error"));
    let bundle = load_fluent_bundles(&lang);

    let services = vec![
        Service {
            name: bundle
                .format_pattern(
                    bundle
                        .get_message("service_3DPrinting")
                        .unwrap()
                        .value()
                        .unwrap(),
                    None,
                    &mut vec![],
                )
                .into(),
            description: bundle
                .format_pattern(
                    bundle
                        .get_message("service_3DPrinting_desc")
                        .unwrap()
                        .value()
                        .unwrap(),
                    None,
                    &mut vec![],
                )
                .into(),
            link: "/fdm-3d-printing".to_string(),
        },
        Service {
            name: bundle
                .format_pattern(
                    bundle
                        .get_message("service_design_optimisation")
                        .unwrap()
                        .value()
                        .unwrap(),
                    None,
                    &mut vec![],
                )
                .into(),
            description: bundle
                .format_pattern(
                    bundle
                        .get_message("service_design_optimisation_desc")
                        .unwrap()
                        .value()
                        .unwrap(),
                    None,
                    &mut vec![],
                )
                .into(),
            link: "/design-optimisation".to_string(),
        },
    ];
    let idx_template = IndexTemplate {
        our_services: bundle
            .format_pattern(
                bundle.get_message("our_services").unwrap().value().unwrap(),
                None,
                &mut vec![],
            )
            .into(),
        services,
        learn_more: bundle
            .format_pattern(
                bundle.get_message("learn_more").unwrap().value().unwrap(),
                None,
                &mut vec![],
            )
            .into(),
    };

    // // Map services to localized versions
    // let localized_services: Vec<Service> = idx_template.services.into_iter().map(|service| {
    //     let name = bundle.get_message(&service.name)
    //         .and_then(|m| m.value())
    //         .map_or_else(|| "Missing Translation".to_string(), |v| bundle.format_pattern(v, None, &mut vec![]).to_string());
    //     let description = bundle.get_message(&service.description)
    //         .and_then(|m| m.value())
    //         .map_or_else(|| "Missing Translation".to_string(), |v| bundle.format_pattern(v, None, &mut vec![]).to_string());

    //     Service { name, description }
    // }).collect();

    // let template = IndexTemplate {
    //     services: localized_services,
    // };

    match idx_template.render() {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .service(fs::Files::new("/static", "static").use_last_modified(true)) // Serve static files
            .route("/", web::get().to(en_redirect)) // Serve the index page
            .route("/{lang}/", web::get().to(index)) // Serve the index page
                                                     // Add other routes and services as needed
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
