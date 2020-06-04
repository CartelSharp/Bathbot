use super::models::Channel;
use crate::util::Error;

use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    Client, Method,
};
use std::fmt::Write;

const BASE_URL: &str = "https://mixer.com/api/v1";

/// API wrapper around the Mixer REST API.
pub struct REST {
    client: Client,
    client_id: String,
}

impl REST {
    pub fn new(client_id: &str) -> Self {
        REST {
            client: Client::new(),
            client_id: client_id.to_string(),
        }
    }

    fn headers(&self, access_token: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(
            HeaderName::from_static("client-id"),
            HeaderValue::from_bytes(self.client_id.as_bytes()).unwrap(),
        );
        if access_token.is_some() {
            map.insert(
                header::AUTHORIZATION,
                HeaderValue::from_bytes(format!("Bearer {}", access_token.unwrap()).as_bytes())
                    .unwrap(),
            );
        }
        map
    }

    /// Result only contains the channels that are __currently__ streaming
    pub async fn channels(&self, user_ids: &[u64]) -> Result<Vec<Channel>, Error> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut r#where = String::from("id:in:{}");
        for id in user_ids {
            let _ = write!(r#where, "{};", id);
        }
        r#where.pop();
        let params = &[("where", r#where.as_str())];
        let response = self
            .query("GET", "channels", Some(params), None, None)
            .await?;
        let channels: Vec<Channel> = serde_json::from_str(&response)?;
        Ok(channels)
    }

    pub async fn channel_by_name(&self, username: &str) -> Result<Channel, Error> {
        let endpoint = format!("channels/{}", username);
        let response = self.query("GET", &endpoint, None, None, None).await?;
        let channel: Channel = serde_json::from_str(&response)?;
        Ok(channel)
    }

    pub async fn channel_by_id(&self, id: u64) -> Result<Channel, Error> {
        let endpoint = format!("channels/{}", id);
        let response = self.query("GET", &endpoint, None, None, None).await?;
        let channel: Channel = serde_json::from_str(&response)?;
        Ok(channel)
    }

    /// Query an endpoint.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP verb
    /// * `endpoint` - API endpoint (do not include the API base URL)
    /// * `params` - query params to include (if none, just send `&[]`)
    /// * `body` - optional HTTP body String
    /// * `access_token` - optional OAuth token
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use mixer_wrappers::REST;
    /// let api = REST::new("");
    /// let text = api.query("GET", "some/endpoint", None, None, None).unwrap();
    /// ```
    async fn query(
        &self,
        method: &str,
        endpoint: &str,
        params: Option<&[(&str, &str)]>,
        body: Option<&str>,
        access_token: Option<&str>,
    ) -> Result<String, Error> {
        let url = format!("{}/{}", BASE_URL, endpoint);
        let method = Method::from_bytes(method.to_uppercase().as_bytes())
            .map_err(|why| Error::Custom(format!("Error creating reqwest method: {}", why)))?;
        debug!("Making {} call to {}", method, url);
        let mut builder = self
            .client
            .request(method, &url)
            .headers(self.headers(access_token));
        if params.is_some() {
            builder = builder.query(params.unwrap());
        }
        if body.is_some() {
            builder = builder.body(body.unwrap().to_owned());
        }
        let req = builder.build()?;
        let resp = self.client.execute(req).await?;
        if resp.status().is_success() {
            Ok(resp.text().await?)
        } else {
            let headers: Vec<_> = resp.headers().iter().map(|h| format!("{:?}", h)).collect();
            let status = resp.status();
            debug!(
                "Got status code {} from endpoint, headers: {}, text: {}",
                status.as_str(),
                headers.join(", "),
                resp.text().await?
            );
            Err(Error::Custom(format!(
                "BadHttpResponseError with code {}",
                status.as_u16()
            )))
        }
    }
}
