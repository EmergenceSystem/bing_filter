use actix_web::{post, App, HttpServer, HttpResponse, Responder};
use scraper::{Html, Selector};
use std::string::String;
use url::form_urlencoded;
use reqwest::Client;
use embryo::{Embryo, EmbryoList};
use serde_json::from_str;
use std::collections::HashMap;

static SEARCH_URL: &str = "https://www.bing.com/search?q=";
static EXCLUDED_CONTENT: [&str; 4] = ["bing.com", "microsoft.com", "ignalez", "bingj.com"];

#[post("/query")]
async fn query_handler(body: String) -> impl Responder {
    let embryo_list = generate_embryo_list(body).await;
    let response = EmbryoList { embryo_list };
    HttpResponse::Ok().json(response)
}

async fn generate_embryo_list(json_string: String) -> Vec<Embryo> {
    let search: HashMap<String,String> = from_str(&json_string).expect("Erreur lors de la désérialisation JSON");
    let encoded_search: String = form_urlencoded::byte_serialize(search.values().next().unwrap().as_bytes()).collect();
    let search_url = format!("{}{}", SEARCH_URL, encoded_search);
    println!("{}", search_url);
    let response = Client::new().get(&search_url).send().await;

    match response {
        Ok(response) => {
            if let Ok(body) = response.text().await {
                let embryo_list = extract_links_from_results(body);
                return embryo_list;
            }
        }
        Err(e) => eprintln!("Error fetching search results: {:?}", e),
    }

    Vec::new()
}

fn extract_links_from_results(html: String) -> Vec<Embryo> {
    let mut embryo_list = Vec::new();
    let fragment = Html::parse_document(&html);
    let selector = Selector::parse("li.b_algo").unwrap();

    for element in fragment.select(&selector) {
        let selector_link = Selector::parse("div a").unwrap();
        let link = element.select(&selector_link).next().and_then(|elem| elem.value().attr("href")).unwrap_or_default().trim().to_string();
        if EXCLUDED_CONTENT.iter().any(|excluded| link.contains(excluded))
            || !link.starts_with("http")
            {
                continue;
            }
        let selector_desc = Selector::parse("div p").unwrap();
        let desc = element.select(&selector_desc)
            .next()
            .map(|elem| elem.text().collect::<Vec<_>>().join(""))
            .unwrap_or_default()
            .trim().to_string();
        
        let icon_text = element.select(&Selector::parse(".algoSlug_icon").unwrap())
            .map(|icon_element| icon_element.text().collect::<String>())
            .collect::<String>();
        let news_dt_text = element.select(&Selector::parse(".news_dt").unwrap())
            .map(|icon_element| icon_element.text().collect::<String>())
            .collect::<String>();

        let resume = desc.replace(&icon_text, "").replace(&news_dt_text, "").replace(" . ", "");

        let embryo = Embryo {
            properties: HashMap::from([
                    ("url".to_string(), link.to_string()),
                    ("resume".to_string(),resume.to_string())])
        };

        embryo_list.push(embryo);
    }

    embryo_list
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    match em_filter::find_port().await {
        Some(port) => {
            let filter_url = format!("http://localhost:{}/query", port);
            println!("Filter registrer: {}", filter_url);
            em_filter::register_filter(&filter_url).await;
            HttpServer::new(|| App::new().service(query_handler))
                .bind(format!("127.0.0.1:{}", port))?.run().await?;
        },
        None => {
            println!("Can't start");
        },
    }
    Ok(())
}

