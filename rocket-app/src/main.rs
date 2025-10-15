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
async fn archidekt(id: &str) -> Template {
    let id = id.to_string();
    let sets = archidekt_provider::get_data(id).await;
    match sets {
        Ok(sets) => {
            Template::render(
                    "sets",
                    context! { sets: sets,title:"Archidekt Set Binder",description:"Set completion for an archidekt collection" },
                )
        }
        Err(err) => Template::render("error", context! {message:err.to_string()}),
    }
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
