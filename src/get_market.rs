use reqwest::blocking::{Client, Response};
use serde_json::json;
use std::error::Error;

// SAFETY: add response status checking and handling for real world usage.
pub fn get_world_market_list(
    region: &str,
    main_cat: u32,
    sub_cat: u32,
) -> Result<Response, Box<dyn Error>> {
    let url = format!(
        "https://{}-trade.naeu.playblackdesert.com/Trademarket/GetWorldMarketList",
        region
    );

    let client = Client::new();
    let response = client
        .post(&url)
        .json(&json!({
            "mainCategory": main_cat,
            "subCategory": sub_cat,
            "keyType": "0"
        }))
        .header("Cache-Control", "no-cache")
        .header("Connection", "close")
        .header("Pragma", "no-cache")
        .header("Content-Type", "application/json")
        .header("User-Agent", "BlackDesert")
        .send()?;

    Ok(response)
}
