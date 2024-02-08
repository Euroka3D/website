use crate::Lang;
use actix_web::{HttpResponse, Responder};
use askama::Template;
use fluent_templates::{static_loader, Loader};
use unic_langid::LanguageIdentifier;

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

// For simplicity, define a struct to represent your services
struct OfferedService {
    name: String,
    description: String,
    link: String,
}

static_loader! {
    static LOCALES = {
        locales: "./locales/",
        fallback_language: "en"
    };
}

fn msg_get<L: AsRef<str>>(title: &str, lang: &L) -> Option<String> {
    LOCALES.lookup(&lang.as_ref().parse().unwrap(), title)
}

pub async fn index(lang: Lang) -> impl Responder {
    let services = vec![
        OfferedService {
            name: msg_get("service_3DPrinting", &lang).unwrap(),
            description: msg_get("service_3DPrinting_desc", &lang).unwrap(),
            link: "/fdm-3d-printing".to_string(),
        },
        OfferedService {
            name: msg_get("service_design_optimisation", &lang).unwrap(),
            description: msg_get("service_design_optimisation_desc", &lang).unwrap(),
            link: "/design-optimisation".to_string(),
        },
    ];
    let idx_template = IndexBodyTemplate {
        our_services: msg_get("our_services", &lang).unwrap(),
        services,
        learn_more: msg_get("learn_more", &lang).unwrap(),
    };

    let page = PageTemplate {
        lang: lang.as_ref().to_string(),
        main: idx_template.to_string(),
        help_section_title: msg_get("footer_help_title", &lang).unwrap(),
        contact_us: msg_get("footer_contact_us", &lang).unwrap(),
    };

    match page.render() {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

pub async fn faq(_lang: Lang) -> impl Responder {
    HttpResponse::Ok().body("raw-content here")
}

#[derive(Template)]
#[template(path = "about.html")]
struct AboutTemplate {
    body_title: String,
    title_byline: String,

    intro_title: String,
    intro_content: String,

    our_approach_title: String,
    our_approach_content: String,

    why_us_title: String,
    why_us_content: String,
}

impl AboutTemplate {
    fn fetch_lang(lang: &Lang) -> Self {
        Self {
            body_title: msg_get("body_title", lang).unwrap(),
            title_byline: msg_get("title_byline", lang).unwrap(),

            intro_title: msg_get("intro_title", lang).unwrap(),
            intro_content: msg_get("intro_content", lang).unwrap(),

            our_approach_title: msg_get("our_approach_title", lang).unwrap(),
            our_approach_content: msg_get("our_approach_content", lang).unwrap(),

            why_us_title: msg_get("why_us_title", lang).unwrap(),
            why_us_content: msg_get("why_us_content", lang).unwrap(),
        }
    }
}
pub async fn about(_lang: Lang) -> impl Responder {
    HttpResponse::Ok().body("raw-content here")
}
