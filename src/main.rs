#![allow(clippy::uninlined_format_args)]
use actix_files as fs;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;
use fluent::{FluentBundle, FluentResource};
use std::borrow::Borrow;
use unic_langid::LanguageIdentifier; // Make sure you include the Template trait

#[derive(Template)]
#[template(path = "page.html")]
struct PageTemplate {
    lang: String,
    main: String,
    help_section_title: String,
    contact_us: String,
}

// Define your Askama template
#[derive(Template)]
#[template(path = "index_body.html")]
struct IndexBodyTemplate {
    our_services: String,
    services: Vec<Service>,
    learn_more: String,
}

async fn faq() -> impl Responder {
    HttpResponse::Ok().body("raw-content here")
}

fn load_fluent_bundles(lang: &LanguageIdentifier) -> FluentBundle<FluentResource> {
    let ftl_path = format!("locales/{}.ftl", lang);
    let ftl_string = std::fs::read_to_string(&ftl_path)
        .unwrap_or_else(|_| panic!("Failed to load FTL file: {}", &ftl_path));
    let resource = FluentResource::try_new(ftl_string)
        .unwrap_or_else(|_| panic!("Failed to parse the FTL file: {}", ftl_path));
    let mut bundle = FluentBundle::new(vec![lang.clone()]);
    bundle
        .add_resource(resource)
        .unwrap_or_else(|_| panic!("Failed to add FTL resource to the bundle: {}", lang));

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

    let msg_get = |title: &str, bundle: &FluentBundle<FluentResource>| {
        bundle
            .format_pattern(
                bundle.get_message(title).unwrap().value().unwrap(),
                None,
                &mut vec![],
            )
            .into()
    };
    let services = vec![
        Service {
            name: msg_get("service_3DPrinting", &bundle),
            description: msg_get("service_3DPrinting_desc", &bundle),
            link: "/fdm-3d-printing".to_string(),
        },
        Service {
            name: msg_get("service_design_optimisation", &bundle),
            description: msg_get("service_design_optimisation_desc", &bundle),
            link: "/design-optimisation".to_string(),
        },
    ];
    let idx_template = IndexBodyTemplate {
        our_services: msg_get("our_services", &bundle),
        services,
        learn_more: msg_get("learn_more", &bundle),
    };

    let page = PageTemplate {
        lang: lang_code.to_string(),
        main: idx_template.to_string(),
        help_section_title: msg_get("footer_help_title", &bundle),
        contact_us: msg_get("footer_contact_us", &bundle),
    };

    match page.render() {
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
            .route("/", web::get().to(en_redirect))
            .route("/{lang}/", web::get().to(index))
            .route("/{lang}/faq", web::get().to(faq))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
