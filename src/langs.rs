use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{
        header::{self, AcceptLanguage, Header, LanguageTag},
        StatusCode,
    },
    FromRequest, HttpMessage, HttpResponse,
};
use futures_util::future::{LocalBoxFuture, TryFutureExt};
use std::future::{ready, Ready};

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Lang {
    En,
    Fr,
    // German
    De,
    He,
}

impl Lang {
    fn from_str<'a>(lang_str: &'a str) -> Result<Self, &'a str> {
        match lang_str {
            "en" => Ok(Lang::En),
            "fr" => Ok(Lang::Fr),
            "de" => Ok(Lang::De),
            "he" => Ok(Lang::He),
            _ => Err(lang_str),
        }
    }
    fn try_from_message<M: HttpMessage>(value: &M) -> Result<Self, String> {
        // todo: check TLD and route first
        let langs: AcceptLanguage = AcceptLanguage::parse(value).map_err(|e| e.to_string())?;
        let mut max = 0.0;
        let mut preferred = Err("".to_string());
        for tag in langs.0.iter().filter(|t| {
            t.item
                .item()
                .map(|t| Lang::from_str(t.primary_language()))
                .is_some()
        }) {
            let qual = tag.quality.to_string().parse().unwrap();
            if qual >= 1.0 {
                return Ok(tag
                    .item
                    .item()
                    .map(LanguageTag::primary_language)
                    .and_then(|s| Lang::from_str(s).ok())
                    .unwrap_or_default());
            }
            if qual > max {
                max = qual;
                let parse = Lang::from_str(tag.item.item().unwrap().primary_language())
                    .map_err(|_| "bad parse".to_string());
                if parse.is_ok() {
                    preferred = parse;
                    max = qual;
                }
            }
        }
        preferred
    }
}

impl AsRef<str> for Lang {
    fn as_ref(&self) -> &str {
        match self {
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::He => "he",
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

        std::future::ready(Ok(Lang::from_str(lang_str).unwrap_or_default()))
    }
}

pub struct LanguageConcierge;

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

pub struct LanguageMiddleware<S> {
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
            .map(Lang::from_str);
        if let Some(Ok(l)) = lang {
            req.extensions_mut().insert(l);
        }
        let resp = self.service.call(req).map_ok(|resp: ServiceResponse<B>| {
            let original_req = resp.request();
            let lang = Lang::try_from_message(original_req).unwrap_or_default();
            let stat = resp.response().status();
            if stat == StatusCode::NOT_FOUND {
                let new_path = format!(
                    "/{}/{}",
                    lang.as_ref(),
                    resp.request().path().trim_start_matches('/')
                );
                let redir_resp = HttpResponse::Found()
                    .insert_header((header::LOCATION, new_path))
                    .finish()
                    .map_into_right_body();
                ServiceResponse::new(resp.into_parts().0, redir_resp)
            } else {
                resp.map_into_left_body()
            }
        });
        Box::pin(resp)
    }
}
