//! Blocking client and picker for the Immich photo search API.
//!
//! The client targets the v1.116.2 search routes and reads the server
//! address from the environment. The picker collects asset IDs across
//! searches and pages forward through random results.

#![warn(missing_docs)]

use reqwest::blocking::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

/// Name of the environment variable holding the API base URL.
pub const BASE_URL_ENV: &str = "IMMICH_BASE_URL";

/// Errors from search requests and response parsing.
#[derive(Debug)]
pub enum Error {
    /// The base URL was missing or invalid.
    Config(String),
    /// The HTTP request failed.
    Http(reqwest::Error),
    /// The response body was not valid search JSON.
    Parse(serde_json::Error),
    /// The server answered with an error status.
    Status(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => write!(f, "invalid configuration: {message}"),
            Self::Http(error) => write!(f, "request failed: {error}"),
            Self::Parse(error) => write!(f, "invalid search response: {error}"),
            Self::Status(code) => write!(f, "server returned status {code}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(error) => Some(error),
            Self::Parse(error) => Some(error),
            Self::Config(_) | Self::Status(_) => None,
        }
    }
}

/// A photo or video asset in the library.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    /// Stable server assigned identifier.
    pub id: String,
    /// Client assigned identifier for the original file.
    #[serde(default)]
    pub device_asset_id: Option<String>,
    /// Identifier of the uploading device.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Media kind reported by the server.
    #[serde(default, rename = "type")]
    pub asset_type: Option<String>,
    /// Original file name reported by the server.
    #[serde(default)]
    pub original_file_name: Option<String>,
    /// Creation timestamp reported by the server.
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Asset hits inside a search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResults {
    /// Number of assets in this page.
    #[serde(default)]
    pub count: u64,
    /// Assets in this page.
    #[serde(default)]
    pub items: Vec<Asset>,
}

/// An album hit inside a search response.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    /// Stable server assigned identifier.
    #[serde(default)]
    pub id: String,
    /// Display name reported by the server.
    #[serde(default)]
    pub name: String,
}

/// Body of the wrapped album group in a search response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AlbumResultsBody {
    /// Number of albums in this page.
    #[serde(default)]
    count: u64,
    /// Albums in this page.
    #[serde(default)]
    items: Vec<Album>,
}

/// Wire shapes accepted for the album group.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AlbumWire {
    /// Object wrapper with items.
    Wrapped(AlbumResultsBody),
    /// Bare album list.
    Bare(Vec<Album>),
}

/// Album hits inside a search response.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AlbumResults {
    /// Number of albums in this page.
    pub count: u64,
    /// Albums in this page.
    pub items: Vec<Album>,
}

impl<'de> Deserialize<'de> for AlbumResults {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Accept a bare list as well so hand written fixtures keep parsing.
        let wire = AlbumWire::deserialize(deserializer)?;
        let results = match wire {
            AlbumWire::Wrapped(body) => Self {
                count: body.count,
                items: body.items,
            },
            AlbumWire::Bare(items) => Self { count: 0, items },
        };
        Ok(results)
    }
}

/// Search response shared by random, smart, and metadata search.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct SearchResponse {
    /// Album hits for the search.
    #[serde(default)]
    pub albums: AlbumResults,
    /// Asset hits for the search.
    pub assets: AssetResults,
}

/// Parse one search response body for any search route.
pub fn parse_search_response(body: &str) -> Result<SearchResponse, Error> {
    serde_json::from_str(body).map_err(Error::Parse)
}

/// Page body for random search.
#[derive(Debug, Serialize)]
struct PageBody {
    /// Page number starting at one.
    page: u64,
}

/// Text query body for smart and metadata search.
#[derive(Debug, Serialize)]
struct QueryBody<'a> {
    /// Free text filter for the search.
    query: &'a str,
}

/// Blocking HTTP client for the Immich search API.
#[derive(Debug)]
pub struct ImmichClient {
    /// Server address without a trailing slash.
    base_url: String,
    /// Underlying connection pool.
    http: HttpClient,
}

impl ImmichClient {
    /// Build a client for the given base URL.
    pub fn new(base_url: &str) -> Self {
        let trimmed = base_url.trim_end_matches('/').to_string();
        Self {
            base_url: trimmed,
            http: HttpClient::new(),
        }
    }

    /// Build a client from the base URL environment variable.
    pub fn from_env() -> Result<Self, Error> {
        // Read the address from the environment so demos avoid flags.
        let raw = env::var(BASE_URL_ENV)
            .map_err(|_| Error::Config(format!("set {BASE_URL_ENV} to the server address")))?;
        if raw.trim().is_empty() {
            return Err(Error::Config(format!(
                "set {BASE_URL_ENV} to the server address"
            )));
        }
        Ok(Self::new(&raw))
    }

    /// Return the base URL in use.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Post a body to one search route and parse the response.
    fn post<R: Serialize>(&self, path: &str, body: &R) -> Result<SearchResponse, Error> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .http
            .post(&url)
            .json(body)
            .send()
            .map_err(Error::Http)?;
        if !response.status().is_success() {
            return Err(Error::Status(response.status().as_u16()));
        }
        // Funnel every route through the shared parser.
        let text = response.text().map_err(Error::Http)?;
        parse_search_response(&text)
    }

    /// Run a random search and return one page of assets.
    pub fn search_random(&self, page: u64) -> Result<SearchResponse, Error> {
        self.post("/search/random", &PageBody { page })
    }

    /// Run a smart search for a text query.
    pub fn search_smart(&self, query: &str) -> Result<SearchResponse, Error> {
        self.post("/search/smart", &QueryBody { query })
    }

    /// Run a metadata search for a text query.
    pub fn search_metadata(&self, query: &str) -> Result<SearchResponse, Error> {
        self.post("/search/metadata", &QueryBody { query })
    }
}

/// Photo picker collecting asset IDs across searches.
#[derive(Debug)]
pub struct Picker {
    /// Picked asset IDs in arrival order.
    selected: Vec<String>,
    /// Page requested by the next random search.
    next_page: u64,
}

impl Picker {
    /// Create an empty picker starting at the first random page.
    pub fn new() -> Self {
        Self {
            selected: Vec::new(),
            next_page: 1,
        }
    }

    /// Feed one search response into the picker selection.
    pub fn add_response(&mut self, response: &SearchResponse) {
        // Push every asset ID so the picker mirrors server order.
        for asset in &response.assets.items {
            self.selected.push(asset.id.clone());
        }
    }

    /// Return the picked asset IDs in arrival order.
    pub fn selection(&self) -> &[String] {
        &self.selected
    }

    /// Return the page the next random search will request.
    pub fn next_page(&self) -> u64 {
        self.next_page
    }

    /// Fetch the next random page and add its assets to the selection.
    pub fn more(&mut self, client: &ImmichClient) -> Result<(), Error> {
        let response = client.search_random(self.next_page)?;
        self.add_response(&response);
        self.next_page += 1;
        Ok(())
    }
}

impl Default for Picker {
    fn default() -> Self {
        Self::new()
    }
}
