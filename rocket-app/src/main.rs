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
        .filter(|set| {
            set.progress.base_count > 0 && set.progress.base_collected == set.progress.base_count
        })
        .count();
    let matched_count: usize = sets.iter().map(|set| set.collected_count).sum();
    let sets: Vec<serde_json::Value> = sets.into_iter().map(|set| set_view(set, false)).collect();
    Template::render(
        "sets",
        context! {
            title: "Your collection · Set Binder", description: "Set completion for your Archidekt collection.",
            collection_id: id, sets, set_count, completed_count, matched_count,
        },
    )
}

fn set_view(mut set: models::Set, include_cards: bool) -> serde_json::Value {
    let available = set.set_count > 0;
    let base_available = set.progress.base_count > 0;
    let missing_count = set
        .progress
        .base_count
        .saturating_sub(set.progress.base_collected);
    let complete = base_available && missing_count == 0;
    let base_completion = set.progress.base_completion();
    let variant_bonus = set.progress.variant_bonus();
    // Both segments share a scale, fitted into the row without clipping bonuses >100%.
    let base_track_width = 100.0 / (1.0 + variant_bonus);
    // Rarity targets describe the full base set, independent of modal filters.
    let rarity_progress: Vec<_> = if include_cards {
        models::Rarity::ALL
            .into_iter()
            .filter_map(|rarity| {
                let cards: Vec<_> = set
                    .cards
                    .iter()
                    .filter(|card| {
                        card.printing_kind == models::PrintingKind::Base && card.rarity == rarity
                    })
                    .collect();
                if cards.is_empty() {
                    return None;
                }
                Some(serde_json::json!({
                    "rarity": rarity, "label": rarity.label(), "total": cards.len(),
                    "collected": cards.iter().filter(|card| card.collected == Some(true)).count(),
                }))
            })
            .collect()
    } else {
        vec![]
    };
    // Ownership is already reflected on each card; never serialize it twice.
    set.collected_cards.clear();
    if !include_cards {
        set.cards.clear();
    }
    let rarity_labels: Vec<_> = set.cards.iter().map(|card| card.rarity.label()).collect();
    let mut value = serde_json::to_value(set).expect("Set contains only serializable fields");
    for (card, label) in value["cards"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(rarity_labels)
    {
        card["rarity_label"] = label.into();
    }
    value["rarity_progress"] = rarity_progress.into();
    value["available"] = available.into();
    value["base_available"] = base_available.into();
    value["base_percentage"] = format!("{:.2}%", base_completion * 100.0).into();
    value["bonus_percentage"] = format!("{:.2}%", variant_bonus * 100.0).into();
    value["base_completion"] = base_completion.into();
    value["base_track_width"] = format!("{base_track_width:.4}").into();
    value["has_bonus"] = (variant_bonus > 0.0).into();
    value["missing_count"] = missing_count.into();
    value["complete"] = complete.into();
    value
}

#[get("/archidekt/<id>/sets/<code>")]
async fn set_details(
    id: &str,
    code: &str,
) -> Result<Template, rocket::response::status::Custom<&'static str>> {
    use rocket::{http::Status, response::status::Custom};
    if id.is_empty()
        || !id.bytes().all(|byte| byte.is_ascii_digit())
        || code.is_empty()
        || code.len() > 16
        || !code.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(Custom(Status::BadRequest, "Invalid collection or set."));
    }
    let set = archidekt_provider::get_set_data(id, code)
        .await
        .map_err(|error| {
            eprintln!("Unable to load set {code} for collection {id}: {error}");
            Custom(
                Status::BadGateway,
                "Unable to load this set. Please try again.",
            )
        })?
        .ok_or(Custom(
            Status::NotFound,
            "This set is no longer in the collection.",
        ))?;
    Ok(Template::render("set-details", set_view(set, true)))
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

const BROWSE_JS: &str = include_str!("../static/browse.mjs");
#[get("/browse.mjs")]
fn browse_js() -> (ContentType, &'static str) {
    (ContentType::JavaScript, BROWSE_JS)
}

#[launch]
fn rocket() -> _ {
    rocket::build().attach(templates()).mount(
        "/",
        routes![
            archidekt,
            set_details,
            index,
            process_url,
            style,
            app_js,
            browse_js
        ],
    )
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
            progress: models::PrintingProgress {
                base_count: total,
                base_collected: collected,
                ..Default::default()
            },
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
        assert!(html.contains("Base complete</span>"));
        assert!(html.contains("value=\"2\" max=\"2\""));
        assert!(!html.contains("max=\"0\""));
        assert!(html.contains("Base sets complete</dt><dd>1"));
    }
    #[get("/compound-fixture")]
    fn compound_fixture() -> Template {
        let make_set = |code: &str, base, owned, variants, bonus, unknown| models::Set {
            code: code.into(),
            name: Some(code.into()),
            cards: vec![],
            collected_cards: vec![],
            set_percentage: "unused".into(),
            set_count: base + variants + unknown,
            collected_count: owned + bonus,
            set_completion: 0.0,
            progress: models::PrintingProgress {
                base_count: base,
                base_collected: owned,
                variant_count: variants,
                variant_collected: bonus,
                unclassified_count: unknown,
                unclassified_collected: 0,
            },
        };
        collection_page(
            "1234",
            vec![
                make_set("incomplete", 5, 4, 5, 1, 0),
                make_set("bonus", 2, 2, 5, 5, 0),
                make_set("unclassified", 0, 0, 0, 0, 3),
            ],
        )
    }
    #[rocket::async_test]
    async fn compound_progress_keeps_base_and_bonus_separate() {
        let client = rocket::local::asynchronous::Client::tracked(
            rocket().mount("/", routes![compound_fixture]),
        )
        .await
        .unwrap();
        let response = client.get("/compound-fixture").dispatch().await;
        assert_eq!(response.status(), rocket::http::Status::Ok);
        let html = response.into_string().await.unwrap();
        assert!(html.contains("80.00%"));
        assert!(html.contains("+20.00%"));
        assert!(
            html.contains("+250.00%"),
            "bonuses must not be capped at 100%"
        );
        assert!(
            html.contains("value=\"4\" max=\"5\""),
            "base progress uses only the base denominator"
        );
        assert!(
            html.contains("--base-track: 28.5714%"),
            "large bonuses fit the row on a shared scale"
        );
        assert!(html.contains("Base sets complete</dt><dd>1"));
        assert!(html.contains("Base classification unavailable"));
        assert!(!html.contains("NaN"));
        let incomplete = html
            .split("data-set-code=\"incomplete\"")
            .nth(1)
            .unwrap()
            .split("<template")
            .next()
            .unwrap();
        assert!(
            incomplete.contains("data-complete=\"false\""),
            "a variant bonus must not finish the base set"
        );
    }
    fn large_set(code: &str, count: usize) -> models::Set {
        let card = models::Card {
            name: "CARD_PAYLOAD_MUST_NOT_BE_IN_OVERVIEW <script>".repeat(3),
            set_code: code.into(),
            collector_number: "1".into(),
            scryfall_id: None,
            collected: Some(false),
            image: Some("https://example.com/card.png".into()),
            printing_kind: models::PrintingKind::Base,
            rarity: models::Rarity::Unknown,
        };
        models::Set {
            code: code.into(),
            name: Some(format!("Set {code}")),
            cards: vec![card; count],
            collected_cards: vec![],
            set_percentage: "0%".into(),
            set_count: count,
            collected_count: 0,
            set_completion: 0.0,
            progress: models::PrintingProgress {
                base_count: count,
                ..Default::default()
            },
        }
    }
    #[get("/large-fixture")]
    fn large_fixture() -> Template {
        collection_page(
            "1234",
            (0..100)
                .map(|index| large_set(&format!("s{index}"), 600))
                .collect(),
        )
    }
    #[get("/details-fixture")]
    fn details_fixture() -> Template {
        let mut set = large_set("tst", 600);
        set.cards[0].scryfall_id = Some("19cba6be-7291-4788-9241-87dad3b68363".into());
        set.cards[0].rarity = models::Rarity::Rare;
        Template::render("set-details", set_view(set, true))
    }
    #[test]
    fn rarity_completion_counts_exact_base_printings_only() {
        use models::{PrintingKind::*, Rarity::*};
        let mut set = large_set("tst", 9);
        for (card, (rarity, kind, collected)) in set.cards.iter_mut().zip([
            (Common, Base, Some(true)),
            (Common, Base, Some(false)),
            (Rare, Base, Some(false)),
            (Rare, Variant, Some(true)),
            (Mythic, Base, None),
            (Mythic, Unclassified, Some(true)),
            (Unknown, Base, Some(true)),
            (Special, Base, Some(true)),
            (Bonus, Variant, Some(true)),
        ]) {
            card.rarity = rarity;
            card.printing_kind = kind;
            card.collected = collected;
        }
        let view = set_view(set.clone(), true);
        assert_eq!(
            view["rarity_progress"],
            serde_json::json!([
                {"rarity":"common", "label":"Common", "total":2, "collected":1},
                {"rarity":"rare", "label":"Rare", "total":1, "collected":0},
                {"rarity":"mythic", "label":"Mythic", "total":1, "collected":0},
                {"rarity":"special", "label":"Special", "total":1, "collected":1},
                {"rarity":"unknown", "label":"Unknown rarity", "total":1, "collected":1},
            ])
        );
        let overview = set_view(set, false);
        assert_eq!(overview["rarity_progress"], serde_json::json!([]));
        assert_eq!(overview["cards"], serde_json::json!([]));
        assert_eq!(
            set_view(large_set("empty", 0), true)["rarity_progress"],
            serde_json::json!([])
        );
    }
    #[rocket::async_test]
    async fn large_collection_overview_omits_card_payloads() {
        let client = rocket::local::asynchronous::Client::tracked(
            rocket().mount("/", routes![large_fixture, details_fixture]),
        )
        .await
        .unwrap();
        let html = client
            .get("/large-fixture")
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap();
        assert!(
            html.len() < 1024 * 1024,
            "60,000 cards must still produce an overview below 1 MiB; got {} bytes",
            html.len()
        );
        assert!(!html.contains("CARD_PAYLOAD_MUST_NOT_BE_IN_OVERVIEW"));
        assert!(!html.contains("<img"));
        assert!(html.contains("data-details-url=\"/archidekt/1234/sets/s0\""));
        let details = client
            .get("/details-fixture")
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap();
        assert!(details.contains("CARD_PAYLOAD_MUST_NOT_BE_IN_OVERVIEW"));
        assert!(details.contains("&lt;script&gt;"));
        assert!(!details.contains("<script>"));
        assert!(details.contains("id=\"card-kind\""));
        assert!(details.contains("id=\"card-rarity\""));
        assert!(details.contains("data-copy-missing disabled"));
        assert!(details.contains("data-copy-fallback hidden"));
        assert!(details.contains("data-rarity=\"rare\""));
        assert!(details.contains("class=\"rarity-label rare\">Rare</span>"));
        assert!(details.contains("Base completion by rarity"));
        assert!(details.contains("<dd>0 / 1</dd>"));
        assert!(details.contains("Unknown rarity"));
        assert_eq!(details.matches("class=\"card-item\"").count(), 600);
        assert_eq!(details.matches("class=\"card-link\"").count(), 1);
        assert!(
            details.contains(
                "href=\"https://scryfall.com/card/19cba6be-7291-4788-9241-87dad3b68363\""
            )
        );
        assert!(details.contains("target=\"_blank\" rel=\"noopener noreferrer\""));
        assert!(details.len() < 1024 * 1024);
    }
    #[rocket::async_test]
    async fn invalid_detail_paths_fail_before_upstream_requests() {
        let client = rocket::local::asynchronous::Client::tracked(rocket())
            .await
            .unwrap();
        for path in [
            "/archidekt/not-an-id/sets/iko",
            "/archidekt/1234/sets/bad-code",
        ] {
            assert_eq!(
                client.get(path).dispatch().await.status(),
                rocket::http::Status::BadRequest
            );
        }
    }
}
