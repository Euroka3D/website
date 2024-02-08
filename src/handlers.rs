use crate::Lang;
use actix_web::{HttpResponse, Responder};
use askama::Template;
use fluent_templates::{static_loader, Loader};

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

pub async fn index(lang: Lang) -> impl Responder {
    let msg_get = |title: &str, lang: &Lang| {
        LOCALES
            .lookup(&lang.as_ref().parse().unwrap(), title)
            .unwrap()
    };
    let services = vec![
        OfferedService {
            name: msg_get("service_3DPrinting", &lang),
            description: msg_get("service_3DPrinting_desc", &lang),
            link: "/fdm-3d-printing".to_string(),
        },
        OfferedService {
            name: msg_get("service_design_optimisation", &lang),
            description: msg_get("service_design_optimisation_desc", &lang),
            link: "/design-optimisation".to_string(),
        },
    ];
    let idx_template = IndexBodyTemplate {
        our_services: msg_get("our_services", &lang),
        services,
        learn_more: msg_get("learn_more", &lang),
    };

    let page = PageTemplate {
        lang: lang.as_ref().to_string(),
        main: idx_template.to_string(),
        help_section_title: msg_get("footer_help_title", &lang),
        contact_us: msg_get("footer_contact_us", &lang),
    };

    match page.render() {
        Ok(rendered) => HttpResponse::Ok().content_type("text/html").body(rendered),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

pub async fn faq(_lang: Lang) -> impl Responder {
    HttpResponse::Ok().body("raw-content here")
}
