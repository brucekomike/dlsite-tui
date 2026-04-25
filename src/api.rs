use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest_cookie_store::CookieStoreMutex;
use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;

use crate::config::cookies_path;

#[derive(Debug, Clone)]
pub struct WorkInfo {
    pub workno: String,
    pub work_name: String,
    pub maker_name: String,
    pub contents: Vec<FileInfo>,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub workno: String,
    pub file_name: String,
    pub file_size: i64,
}

pub struct ApiClient {
    client: Client,
    cookie_store: Arc<CookieStoreMutex>,
}

impl ApiClient {
    pub fn new() -> Result<Self> {
        let cookies_path = cookies_path();
        let cookie_store = if cookies_path.exists() {
            let data = fs::read_to_string(&cookies_path)?;
            let store = cookie_store::CookieStore::load_json(data.as_bytes())
                .map_err(|e| anyhow!("Failed to load cookies: {}", e))?;
            Arc::new(CookieStoreMutex::new(store))
        } else {
            Arc::new(CookieStoreMutex::new(cookie_store::CookieStore::default()))
        };

        let client = Client::builder()
            .cookie_provider(cookie_store.clone())
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        Ok(Self { client, cookie_store })
    }

    pub fn is_logged_in(&self) -> bool {
        let store = self.cookie_store.lock().unwrap();
        let result = store.iter_any().any(|c| {
            c.name() == "ci_session" || c.name() == "INT_MEMBER_LOGIN"
        });
        result
    }

    pub fn refresh_csrf(&self) -> Result<String> {
        let resp = self
            .client
            .get("https://login.dlsite.com/login")
            .send()
            .context("Failed to GET login page")?;
        let body = resp.text()?;

        if let Some(token) = extract_csrf_from_html(&body) {
            return Ok(token);
        }

        {
            let store = self.cookie_store.lock().unwrap();
            for cookie in store.iter_any() {
                if cookie.name() == "XSRF-TOKEN" {
                    let value = cookie.value().to_string();
                    let decoded = urlencoding_decode(&value);
                    return Ok(decoded);
                }
            }
        }

        Err(anyhow!("Could not find CSRF token"))
    }

    pub fn login(&self, username: &str, password: &str) -> Result<()> {
        let _ = self
            .client
            .get("https://www.dlsite.com/maniax/login/=/skip_register/1")
            .send();

        let token = self.refresh_csrf()?;

        let params = [
            ("login_id", username),
            ("password", password),
            ("_token", token.as_str()),
        ];

        let resp = self
            .client
            .post("https://login.dlsite.com/login")
            .form(&params)
            .send()
            .context("Login POST failed")?;

        let url = resp.url().to_string();
        let body = resp.text().unwrap_or_default();

        if url.contains("/login") && (body.contains("ログイン") || body.contains("login")) && !body.contains("logout") {
            return Err(anyhow!("Login failed - check credentials"));
        }

        Ok(())
    }

    pub fn save_cookies(&self) -> Result<()> {
        let path = cookies_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let store = self.cookie_store.lock().unwrap();
        let mut buf = Vec::new();
        store
            .save_json(&mut buf)
            .map_err(|e| anyhow!("Failed to save cookies: {}", e))?;
        fs::write(&path, &buf)?;
        Ok(())
    }

    pub fn fetch_library_count(&self) -> Result<(u64, u64)> {
        #[derive(Deserialize)]
        struct CountResp {
            user: u64,
            page_limit: u64,
        }
        let resp: CountResp = self
            .client
            .get("https://play.dlsite.com/api/v3/content/count")
            .send()
            .context("Failed to fetch library count")?
            .json()
            .context("Failed to parse library count")?;
        Ok((resp.user, resp.page_limit))
    }

    pub fn fetch_purchased_page(&self, page: u64) -> Result<Vec<(String, String)>> {
        #[derive(Deserialize)]
        struct Item {
            workno: String,
            #[serde(default)]
            sales_date: String,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Resp {
            Wrapped { works: Vec<Item> },
            Array(Vec<Item>),
        }

        let resp = self
            .client
            .get(format!("https://play.dlsite.com/api/v3/content/sales?page={}", page))
            .send()
            .context("Failed to fetch purchased page")?;
        
        let resp_text = resp.text()?;
        let parsed: Resp = serde_json::from_str(&resp_text)
            .context("Failed to parse purchased page")?;

        let items = match parsed {
            Resp::Wrapped { works } => works,
            Resp::Array(arr) => arr,
        };

        Ok(items.into_iter().map(|i| (i.workno, i.sales_date)).collect())
    }

    pub fn fetch_work_info(&self, id: &str) -> Result<WorkInfo> {
        #[derive(Deserialize)]
        struct RawFileInfo {
            #[serde(default)]
            workno: String,
            #[serde(default)]
            file_name: String,
            #[serde(default)]
            file_size: i64,
        }

        #[derive(Deserialize)]
        struct RawWorkInfo {
            workno: String,
            work_name: String,
            #[serde(default)]
            maker_name: String,
            #[serde(default)]
            contents: Vec<RawFileInfo>,
        }

        let url = format!(
            "https://www.dlsite.com/maniax/api/=/product.json?workno={}",
            id
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .context("Failed to fetch work info")?;

        let arr: Vec<RawWorkInfo> = resp.json().context("Failed to parse work info")?;
        let raw = arr.into_iter().next().ok_or_else(|| anyhow!("Empty work info response"))?;

        Ok(WorkInfo {
            workno: raw.workno,
            work_name: raw.work_name,
            maker_name: raw.maker_name,
            contents: raw
                .contents
                .into_iter()
                .map(|f| FileInfo {
                    workno: f.workno,
                    file_name: f.file_name,
                    file_size: f.file_size,
                })
                .collect(),
        })
    }

    pub fn download_file(
        &self,
        work_id: &str,
        file_number: usize,
        file_name: &str,
        output_dir: &Path,
    ) -> Result<u64> {
        let url = format!(
            "https://www.dlsite.com/maniax/download/=/number/{}/product_id/{}.html",
            file_number, work_id
        );

        let mut resp = self
            .client
            .get(&url)
            .send()
            .context("Download request failed")?;

        fs::create_dir_all(output_dir)?;
        let out_path = output_dir.join(file_name);
        let mut file = fs::File::create(&out_path)?;
        let mut buf = Vec::new();
        resp.read_to_end(&mut buf)?;
        file.write_all(&buf)?;
        Ok(buf.len() as u64)
    }
}

fn extract_csrf_from_html(body: &str) -> Option<String> {
    let needle = r#"name="_token" value=""#;
    if let Some(start) = body.find(needle) {
        let rest = &body[start + needle.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    let needle2 = r#"name="_token""#;
    if let Some(pos) = body.find(needle2) {
        let rest = &body[pos..];
        let value_needle = r#"value=""#;
        if let Some(vstart) = rest.find(value_needle) {
            let vrest = &rest[vstart + value_needle.len()..];
            if let Some(vend) = vrest.find('"') {
                return Some(vrest[..vend].to_string());
            }
        }
    }
    let meta_needle = r#"name="csrf-token" content=""#;
    if let Some(start) = body.find(meta_needle) {
        let rest = &body[start + meta_needle.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{}{}", h1, h2), 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}
