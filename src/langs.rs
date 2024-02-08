use crate::Lang;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{header, StatusCode},
    FromRequest, HttpMessage, HttpResponse,
};
use futures_util::future::{LocalBoxFuture, TryFutureExt};
use std::future::{ready, Ready};

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
            "he" => Ok(Lang::He),
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
            .map(Lang::try_from);
        if let Some(Ok(l)) = lang {
            req.extensions_mut().insert(l);
        }
        let resp = self.service.call(req).map_ok(|resp: ServiceResponse<B>| {
            let original_req = resp.request();
            let stat = resp.response().status();
            if stat == StatusCode::NOT_FOUND {
                let lang = original_req
                    .headers()
                    .get(&header::ACCEPT_LANGUAGE)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|vs| Lang::from_accept_lang_header(vs).ok())
                    .unwrap_or_default();
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
