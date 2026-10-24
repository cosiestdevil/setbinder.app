use rocket::fairing::Fairing;
use rocket::{form::Form, http::ContentType, response::Redirect};
#[cfg(not(debug_assertions))]
use rocket_dyn_templates::TemplateInfo;
use rocket_dyn_templates::{Template, context};

#[macro_use]
extern crate rocket;

#[get("/")]
async fn index() -> Template {
    Template::render(
        "index",
        context! {title:"Set Binder",description:"Track every set. Complete your collection."},
    )
}
#[post("/", data = "<url>")]
async fn process_url(url: Form<&str>) -> Redirect {
    regex::Regex::new(r"https://archidekt\.com/collection/v2/(\d+)/?")
        .unwrap()
        .captures(&url)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str())
        .map(|id| Redirect::to(format!("/archidekt/{id}")))
        .unwrap_or_else(|| Redirect::to("/"))
}
#[get("/archidekt/<id>")]
async fn archidekt(id: &str) -> Result<Template, rocket::response::status::Custom<&'static str>> {
    use rocket::{http::Status, response::status::Custom};
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Custom(
            Status::BadRequest,
            "Enter a numeric Archidekt collection ID.",
        ));
    }
    let sets = archidekt_provider::get_data(id).await.map_err(|error| {
        eprintln!("Unable to load collection {id}: {error}");
        match &error {
            provider::Error::Http(http) if http.status().is_some_and(|status| matches!(status.as_u16(), 401 | 403 | 404)) =>
                Custom(Status::NotFound, "Collection unavailable. Check that the ID is correct and the collection is public."),
            _ => Custom(Status::BadGateway, "Unable to load collection data. Please try again later."),
        }
    })?;
    Ok(Template::render(
        "sets",
        context! { sets: sets,title:"Archidekt Set Binder",description:"Set completion for an archidekt collection" },
    ))
}

const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/styles.min.css"));
#[get("/style.css")]
fn style() -> (ContentType, &'static str) {
    (ContentType::CSS, CSS)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(templates())
        .mount("/", routes![archidekt, index, process_url, style])
}
#[cfg(debug_assertions)]
fn templates() -> impl Fairing {
    println!("Debug Templates");
    Template::fairing()
}
#[cfg(not(debug_assertions))]
fn templates() -> impl Fairing {
    println!("Embedded Templates");
    Template::custom(|(engines, templates)| {
        for (name, content) in templates::TEMPLATES {
            engines
                .handlebars
                .register_template_string(name, content)
                .unwrap();
            templates.insert(
                name.to_string(),
                TemplateInfo {
                    path: None,
                    engine_ext: "hbs",
                    data_type: ContentType::HTML,
                },
            );
        }
    })
}

#[cfg(not(debug_assertions))]
mod templates {
    include!(concat!(env!("OUT_DIR"), "/templates.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[rocket::async_test]
    async fn invalid_collection_id_returns_helpful_bad_request() {
        let client = rocket::local::asynchronous::Client::tracked(rocket())
            .await
            .unwrap();
        let response = client.get("/archidekt/not-a-number").dispatch().await;
        assert_eq!(response.status(), rocket::http::Status::BadRequest);
        assert!(
            response
                .into_string()
                .await
                .unwrap()
                .contains("numeric Archidekt collection ID")
        );
    }
}
