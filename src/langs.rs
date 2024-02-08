use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::{
        header::{self, AcceptLanguage, Header, HeaderMap},
        StatusCode,
    },
    FromRequest, HttpResponse, HttpMessage,
};
use futures_util::future::{LocalBoxFuture, TryFutureExt};
use std::future::{ready, Ready};

#[derive(Debug, Hash, Eq, PartialEq)]
enum Lang {
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

impl<M: HttpMessage> From<&M> for Lang {
    fn from(value: &M) -> Self {
        // todo: check TLD and route first
        let Ok(accepts) = value.headers().get(header::ACCEPT_LANGUAGE).map(AcceptLanguage::parse) else {
                return Lang::default();
            };
        let max = 0.0;
        let preferred = None;
        for tag in accepts.iter().filter(|t| t.item.item().map(|t| Lang::from(t.primary_language()))) {
            let qual = tag.quality().parse().unwrap();
            if qual >= 1.0 {
                return tag.item.item().primary_language().try_into().unwrap_or_default();
            }
            if qual > max {
                preferred = Some(tag.item.item().unwrap().try_into().unwrap())
            }
        }
        preferred.unwrap_or_default()
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
            let lang = Lang::from(*original_req);
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
