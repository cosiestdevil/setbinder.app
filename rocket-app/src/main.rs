use rocket::fairing::Fairing;
use rocket::{form::Form, http::ContentType, response::Redirect};
#[cfg(not(debug_assertions))]
use rocket_dyn_templates::TemplateInfo;
use rocket_dyn_templates::{Template, context};

#[macro_use]
extern crate rocket;

#[get("/")]
fn index() -> Template {
    entry_page("", None)
}

fn entry_page(url: &str, error: Option<&str>) -> Template {
    Template::render(
        "index",
        context! {
            title: "Set Binder · A home for every printing",
            description: "See your Magic collection by set. Track your progress and find missing printings with your public Archidekt collection.",
            url, error,
        },
    )
}

#[derive(FromForm)]
struct CollectionForm {
    url: String,
}

#[post("/", data = "<form>")]
fn process_url(
    form: Form<CollectionForm>,
) -> Result<Redirect, rocket::response::status::Custom<Template>> {
    let url = form.url.trim();
    let regex = regex::Regex::new(
        r"^https?://(?:www\.)?archidekt\.com/collection/v2/([0-9]+)/?(?:[?#].*)?$",
    )
    .unwrap();
    if let Some(captures) = regex.captures(url) {
        Ok(Redirect::to(format!("/archidekt/{}", &captures[1])))
    } else {
        Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            entry_page(
                url,
                Some(
                    "Use a collection link such as https://archidekt.com/collection/v2/1234. Check that you copied the collection URL, rather than a deck link.",
                ),
            ),
        ))
    }
}

fn error_page(
    status: rocket::http::Status,
    heading: &str,
    message: &str,
    retry_url: Option<String>,
) -> rocket::response::status::Custom<Template> {
    rocket::response::status::Custom(
        status,
        Template::render(
            "error",
            context! {
                title: "Collection unavailable · Set Binder", description: "Help loading your Archidekt collection.", heading, message, retry_url,
            },
        ),
    )
}

#[get("/archidekt/<id>")]
async fn archidekt(id: &str) -> Result<Template, rocket::response::status::Custom<Template>> {
    use rocket::http::Status;
    if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(error_page(
            Status::BadRequest,
            "Let's find your collection.",
            "Enter a numeric Archidekt collection ID, or paste your collection link on the home page.",
            None,
        ));
    }
    let sets = archidekt_provider::get_data(id).await.map_err(|error| {
        eprintln!("Unable to load collection {id}: {error}");
        let unavailable = matches!(&error, provider::Error::Http(http)
            if http.url().and_then(|url| url.host_str()) == Some("archidekt.com")
                && http.status().is_some_and(|status| matches!(status.as_u16(), 401 | 403 | 404)));
        if unavailable {
            error_page(Status::NotFound, "This binder isn't available.", "Check that the collection ID is correct and your Archidekt collection is public.", None)
        } else {
            error_page(Status::BadGateway, "Your binder needs a moment.", "We couldn't load collection data from Archidekt or Scryfall. Please try again in a moment.", Some(format!("/archidekt/{id}")))
        }
    })?;
    Ok(collection_page(id, sets))
}

fn collection_page(id: &str, sets: Vec<models::Set>) -> Template {
    let set_count = sets.len();
    let completed_count = sets
        .iter()
        .filter(|set| set.set_count > 0 && set.collected_count == set.set_count)
        .count();
    let matched_count: usize = sets.iter().map(|set| set.collected_count).sum();
    let sets: Vec<serde_json::Value> = sets
        .into_iter()
        .map(|set| {
            let available = set.set_count > 0;
            let missing_count = set.set_count.saturating_sub(set.collected_count);
            let complete = available && missing_count == 0;
            let mut value =
                serde_json::to_value(set).expect("Set contains only serializable fields");
            value["available"] = available.into();
            value["missing_count"] = missing_count.into();
            value["complete"] = complete.into();
            value
        })
        .collect();
    Template::render(
        "sets",
        context! {
            title: "Your collection · Set Binder", description: "Set completion for your Archidekt collection.",
            collection_id: id, sets, set_count, completed_count, matched_count,
        },
    )
}

const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/styles.min.css"));
#[get("/style.css")]
fn style() -> (ContentType, &'static str) {
    (ContentType::CSS, CSS)
}

const APP_JS: &str = include_str!("../static/app.js");
#[get("/app.js")]
fn app_js() -> (ContentType, &'static str) {
    (ContentType::JavaScript, APP_JS)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(templates())
        .mount("/", routes![archidekt, index, process_url, style, app_js])
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
    #[rocket::async_test]
    async fn collection_form_preserves_invalid_input_and_accepts_public_urls() {
        use rocket::http::{ContentType, Status};
        let client = rocket::local::asynchronous::Client::tracked(rocket())
            .await
            .unwrap();
        let response = client
            .post("/")
            .header(ContentType::Form)
            .body("url=https%3A%2F%2Farchidekt.com%2Fdecks%2F1234")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);
        let html = response.into_string().await.unwrap();
        assert!(html.contains("https://archidekt.com/decks/1234"));
        assert!(html.contains("aria-invalid=\"true\""));
        let response = client
            .post("/")
            .header(ContentType::Form)
            .body("url=https%3A%2F%2Farchidekt.com%2Fcollection%2Fv2%2F1234%2F")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/archidekt/1234")
        );
    }

    #[get("/fixture")]
    fn fixture() -> Template {
        let make_set = |code: &str, total, collected| models::Set {
            code: code.into(),
            name: Some(format!("Set {code}")),
            cards: vec![],
            collected_cards: vec![],
            set_percentage: "100.00%".into(),
            set_count: total,
            collected_count: collected,
            set_completion: 1.0,
        };
        collection_page(
            "1234",
            vec![make_set("known", 2, 2), make_set("unknown", 0, 0)],
        )
    }

    #[rocket::async_test]
    async fn renders_completion_and_unavailable_states() {
        let client =
            rocket::local::asynchronous::Client::tracked(rocket().mount("/", routes![fixture]))
                .await
                .unwrap();
        let response = client.get("/fixture").dispatch().await;
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let html = response.into_string().await.unwrap();
        assert!(html.contains("Set data unavailable"));
        assert!(html.contains("Complete</span>"));
        assert!(html.contains("value=\"2\" max=\"2\""));
        assert!(!html.contains("max=\"0\""));
        assert!(html.contains("Sets complete</dt><dd>1"));
    }
}
