#![allow(clippy::uninlined_format_args)]
use actix_files as fs;
use actix_web::{http, middleware, web, App, FromRequest, HttpResponse, HttpServer, Responder};
use askama::Template;
use fluent::{FluentBundle, FluentResource};
use std::future::Ready;
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
    services: Vec<OfferedService>,
    learn_more: String,
}

async fn faq(_lang: Lang) -> impl Responder {
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
struct OfferedService {
    name: String,
    description: String,
    link: String,
}

async fn index(lang: Lang) -> impl Responder {
    // TODO: custom extractor

    let bundle = load_fluent_bundles(&lang.as_ref().parse().unwrap());

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
        OfferedService {
            name: msg_get("service_3DPrinting", &bundle),
            description: msg_get("service_3DPrinting_desc", &bundle),
            link: "/fdm-3d-printing".to_string(),
        },
        OfferedService {
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
        lang: lang.as_ref().to_string(),
        main: idx_template.to_string(),
        help_section_title: msg_get("footer_help_title", &bundle),
        contact_us: msg_get("footer_contact_us", &bundle),
    };

    match page.render() {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

#[derive(Debug)]
enum Lang {
    En,
    Fr,
    // German
    De,
}

impl Lang {
    fn from_accept_lang_header(langs_str: &str) -> Result<Lang, ()> {
        // iterate over the languages...
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

impl AsRef<str> for Lang {
    fn as_ref(&self) -> &str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::De => "de",
        }
    }
}
impl<'a> TryFrom<&'a str> for Lang {
    type Error = &'a str;

    fn try_from(lang_str: &'a str) -> Result<Self, Self::Error> {
        log::info!("entered try-from string to lang {}", lang_str);
        // from the `-` onwards: get rid of it. we don't care about region
        let lang = match dbg!(lang_str.split_once('-')) {
            Some((lang, _region)) => lang,
            _ => lang_str,
        };

        log::info!("lang: {}", lang);

        match lang {
            "en" => Ok(Lang::En),
            "fr" => Ok(Lang::Fr),
            "de" => Ok(Lang::De),
            _ => Err(lang_str),
        }
    }
}

impl Default for Lang {
    fn default() -> Self {
        Self::En
    }
}

impl FromRequest for Lang {
    type Error = Box<dyn std::error::Error>;

    type Future = Ready<Result<Lang, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        log::info!("entered from-request");
        let Some(lang_str) = req.match_info().get("lang") else {
            return std::future::ready(Ok(Lang::default()));
        };

        std::future::ready(Ok(Lang::try_from(lang_str).unwrap_or_default()))
    }
}
async fn prefix_fallback_lang(req: actix_web::HttpRequest) -> impl Responder {
    log::info!("entered prefix fallback");
    let path = req.path().trim_start_matches('/');
    let lang_str = req
        .headers()
        .get("Accept-Language")
        .and_then(|hv| hv.to_str().ok())
        .unwrap_or_default();
    let Ok(lang) = Lang::from_accept_lang_header(lang_str) else {
        return HttpResponse::InternalServerError().finish();
    };

    log::info!("end of prefix addition: {:#?}", &lang);
    let new_path = format!("/{}/{}", lang.as_ref(), path);
    HttpResponse::Found()
        .append_header((http::header::LOCATION, new_path))
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .service(fs::Files::new("/static", "static").use_last_modified(true)) // Serve static files
            .service(
                web::scope("/{lang}")
                    .route("", web::get().to(index))
                    .route("/faq", web::get().to(faq)),
            )
            .default_service(web::get().to(prefix_fallback_lang))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
