use anyhow::{Context, Result};
use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::Event;
use reqwest::Method;
use tracing::{info, instrument};

use crate::sync::{Client, Error, FileError, FileResult, LocalFileData};

#[derive(Debug)]
pub struct WebDavClient {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
    remote_path: String,
}

impl WebDavClient {
    #[allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]
    pub fn new(
        base_url: String,
        username: String,
        password: String,
        remote_path: String,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .context("Failed to build the WebDAV HTTP client")?;

        let base_url = base_url.trim_end_matches('/').to_string();
        let remote_path = if remote_path.starts_with('/') {
            remote_path
        } else {
            format!("/{remote_path}")
        };
        let remote_path = remote_path.trim_end_matches('/').to_string();

        Ok(Self {
            client,
            base_url,
            username,
            password,
            remote_path,
        })
    }

    fn file_url(&self, filename: &str) -> String {
        format!("{}{}/{}", self.base_url, self.remote_path, filename)
    }

    fn dir_url(&self) -> String {
        format!("{}{}/", self.base_url, self.remote_path)
    }

    #[allow(
        clippy::unwrap_used,
        reason = "MKCOL is a valid HTTP method; from_bytes only fails on invalid tokens"
    )]
    async fn ensure_dir_exists(&self) -> Result<(), Error> {
        let resp = self
            .client
            .request(Method::from_bytes(b"MKCOL").unwrap(), self.dir_url())
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|err| Error::Internal(Box::new(err)))?;

        let status = resp.status();
        if status.is_success() || status == 405 || status == 301 {
            Ok(())
        } else {
            Err(Error::Internal(Box::new(std::io::Error::other(format!(
                "MKCOL failed with status: {status}"
            )))))
        }
    }

    #[allow(
        clippy::unwrap_used,
        reason = "PROPFIND is a valid HTTP method; from_bytes only fails on invalid tokens"
    )]
    async fn list_json_files(&self) -> Result<Vec<String>, Error> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;

        let resp = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), self.dir_url())
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await
            .map_err(|err| Error::Internal(Box::new(err)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(Error::Internal(Box::new(std::io::Error::other(format!(
                "PROPFIND failed with status: {status}"
            )))));
        }

        let xml = resp
            .text()
            .await
            .map_err(|err| Error::Internal(Box::new(err)))?;

        parse_json_filenames_from_propfind(&xml, &self.remote_path)
    }
}

fn parse_json_filenames_from_propfind(xml: &str, remote_path: &str) -> Result<Vec<String>, Error> {
    let mut reader = Reader::from_str(xml);
    let mut filenames = Vec::new();
    let mut in_href = false;
    let mut current_href = String::new();
    let dir_prefix = format!("{remote_path}/");

    loop {
        match reader.read_event() {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if local_name == "href" {
                    in_href = true;
                    current_href.clear();
                }
            }
            Ok(Event::Text(e)) if in_href => {
                current_href.push_str(&e.unescape().map_err(|err| Error::Internal(Box::new(err)))?);
            }
            Ok(Event::End(e)) => {
                let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                if local_name == "href" {
                    in_href = false;
                    // href is a full URL; extract the filename after remote_path
                    if let Some(pos) = current_href.rfind(&dir_prefix) {
                        let after = &current_href[pos + dir_prefix.len()..];
                        let name = after.trim_end_matches('/');
                        if !name.is_empty()
                            && name.to_lowercase().ends_with(".json")
                            && !name.contains('/')
                        {
                            filenames.push(name.to_string());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => return Err(Error::Internal(Box::new(err))),
            _ => {}
        }
    }

    Ok(filenames)
}

#[async_trait]
impl Client for WebDavClient {
    #[instrument(skip_all)]
    async fn sync_file(&self, data: LocalFileData) -> Result<(), FileError> {
        info!("Syncing file via WebDAV: {}", data.path.display());

        self.ensure_dir_exists()
            .await
            .map_err(|err| FileError::from_error(&data.filename, err))?;

        let url = self.file_url(&data.filename);
        let resp = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/json")
            .body(data.body)
            .send()
            .await
            .map_err(|err| FileError::from_error(&data.filename, err.into()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(FileError::from_error(
                &data.filename,
                Error::Internal(Box::new(std::io::Error::other(format!(
                    "WebDAV PUT failed with status: {status}"
                )))),
            ));
        }

        info!("File synced via WebDAV: {}", data.path.display());
        Ok(())
    }

    #[instrument(skip_all)]
    async fn load_files(&self) -> Result<Box<dyn Iterator<Item = FileResult>>, Error> {
        let filenames = self.list_json_files().await?;

        let dir_url = self.dir_url();
        let username = self.username.clone();
        let password = self.password.clone();

        let mut results: Vec<FileResult> = Vec::new();
        for filename in filenames {
            let url = format!("{dir_url}{filename}");
            let resp = self
                .client
                .get(&url)
                .basic_auth(&username, Some(&password))
                .send()
                .await
                .map_err(|err| Error::Internal(Box::new(err)))?;

            if !resp.status().is_success() {
                let status = resp.status();
                results.push(Err(FileError::from_error(
                    &filename,
                    Error::Internal(Box::new(std::io::Error::other(format!(
                        "WebDAV GET failed with status: {status}"
                    )))),
                )));
                continue;
            }

            let content = resp
                .text()
                .await
                .map_err(|err| Error::Internal(Box::new(err)))?;

            results.push(Ok((filename, content)));
        }

        Ok(Box::new(results.into_iter()))
    }
}
