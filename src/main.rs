#![allow(clippy::uninlined_format_args)]
use actix_files as fs;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{
        self,
        header::{self, HeaderValue},
        StatusCode,
    },
    middleware, web, App, FromRequest, HttpMessage, HttpResponse, HttpServer, Responder,
};
use askama::Template;
use fluent::{FluentBundle, FluentResource};
use futures_util::{
    future::{LocalBoxFuture, TryFutureExt},
    FutureExt,
};
use std::future::{ready, Ready};
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
        // from the `-` onwards: get rid of it. we don't care about region
        let lang = match lang_str.split_once('-') {
            Some((lang, _region)) => lang,
            _ => lang_str,
        };

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
        let Some(lang_str) = req.match_info().get("lang") else {
            return std::future::ready(Ok(Lang::default()));
        };

        std::future::ready(Ok(Lang::try_from(lang_str).unwrap_or_default()))
    }
}

struct LanguageConcierge;

impl<S, B> Transform<S, ServiceRequest> for LanguageConcierge
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;

    type Error = actix_web::Error;

    type Transform = LanguageMiddleware<S>;

    type InitError = ();

    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(LanguageMiddleware { service }))
    }
}

struct LanguageMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for LanguageMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        let lang = req
            .path()
            .trim_start_matches('/')
            .split('/')
            .next()
            .map(Lang::try_from);
        if let Some(Ok(l)) = lang {
            req.extensions_mut().insert(l);
        }
        let resp = self.service.call(req).map_ok(|resp: ServiceResponse<B>| {
            let stat = resp.response().status();
            if stat == StatusCode::NOT_FOUND {
                let new_path = format!("/en/{}", resp.request().path().trim_start_matches('/'));
                let redir_resp = HttpResponse::Found()
                    .insert_header((header::LOCATION, new_path))
                    .finish()
                    .map_into_right_body();
                ServiceResponse::new(resp.into_parts().0, redir_resp)
            } else {
                resp.map_into_left_body()
            }
        });
        Box::pin(async move { resp.await })
        //    // do we have a good lang code?
        //    let lang = Lang::try_from(lang_code);
        //    // no: extract from accept-language...
        //    if lang.is_err() {
        //        let lang_str = req
        //            .headers()
        //            .get("Accept-Language")
        //            .and_then(|hv| hv.to_str().ok())
        //            .unwrap_or_default();
        //        let Ok(lang) = Lang::from_accept_lang_header(lang_str) else {
        //            todo!("handle no lang, no accept language");
        //        };

        //        let new_path = format!("/{}/{}", lang.as_ref(), req.path().trim_start_matches("/"));
        //        let resp = req.into_response(
        //            HttpResponse::Found()
        //                .append_header((header::LOCATION, new_path))
        //                .finish()
        //                .map_into_right_body(),
        //        );

        //        Box::pin(async { Ok(resp) })
        //    } else {
        //        self.service.call(req).map_ok(ServiceResponse::map_into_left_body).boxed_local()
        //    }
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
            .wrap(middleware::NormalizePath::trim())
            .wrap(LanguageConcierge)
            .service(
                web::scope("/{lang}")
                    .route("", web::get().to(index))
                    .route("/", web::get().to(index))
                    .route("/faq", web::get().to(faq)),
            )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
