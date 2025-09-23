use rocket::{form::Form, http::ContentType, response::Redirect};
use rocket_dyn_templates::{Template, context};

#[macro_use]
extern crate rocket;

#[get("/")]
async fn index() -> Template {
    Template::render("index", context! {title:"Set Binder",description:"Track every set. Complete your collection."})
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
    let sets = archidekt_provider::get_data(id).await;
    Template::render("sets", context! { sets: sets,title:"Archidekt Set Binder",description:"Set completion for an archidekt collection" })
}



const CSS: &str = include_str!(concat!(env!("OUT_DIR"), "/styles.min.css"));
#[get("/style.css")]
fn style() -> (ContentType, &'static str) {
    (ContentType::CSS, CSS)
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::fairing())
        .mount("/", routes![archidekt, index, process_url, style])
}
