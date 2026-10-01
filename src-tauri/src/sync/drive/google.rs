//! Real Google Drive REST v3 client (Phase A / "#1") implementing [`DriveApi`] over
//! `reqwest`. Used by the desktop backup path to upload to the OWNER'S OWN Drive with
//! the minimal `drive.file` scope (only files the app itself creates). The sync trait
//! is served over an owned current-thread Tokio runtime, so **call it from a blocking
//! context** (A3 runs the backup via `spawn_blocking`; never from inside another
//! `block_on`). The bearer token is fetched once per run by [`super::oauth`] and is
//! valid ~1 hour — longer than any backup — so no mid-run refresh is needed.
//!
//! Endpoints (Google Drive API v3, stable):
//! * list/metadata/patch/delete  `https://www.googleapis.com/drive/v3/files`
//! * download                    `.../files/{id}?alt=media`
//! * upload (multipart/related)  `https://www.googleapis.com/upload/drive/v3/files`

use super::{DriveApi, DriveError, DriveFile, DriveResult};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::BTreeMap;

const API: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const FIELDS: &str = "id,name,mimeType,size,md5Checksum,parents,appProperties";

/// A live Drive client bound to a single acting identity (its access token).
pub struct GoogleDrive {
    http: reqwest::Client,
    rt: tokio::runtime::Runtime,
    access_token: String,
}

impl GoogleDrive {
    /// Build a client from a valid OAuth access token (see
    /// [`super::oauth::valid_access_token`]).
    pub fn new(access_token: String) -> DriveResult<Self> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| DriveError::Io(e.to_string()))?;
        Ok(Self { http: reqwest::Client::new(), rt, access_token })
    }
}

// ---- Drive JSON shapes -----------------------------------------------------

#[derive(Deserialize)]
struct ApiFile {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "mimeType")]
    mime_type: String,
    #[serde(default)]
    size: Option<String>,
    #[serde(default, rename = "md5Checksum")]
    md5_checksum: Option<String>,
    #[serde(default)]
    parents: Option<Vec<String>>,
    #[serde(default, rename = "appProperties")]
    app_properties: Option<BTreeMap<String, String>>,
}

impl ApiFile {
    fn into_drive(self) -> DriveFile {
        DriveFile {
            id: self.id,
            name: self.name,
            parent: self.parents.and_then(|p| p.into_iter().next()),
            is_folder: self.mime_type == FOLDER_MIME,
            size: self.size.and_then(|s| s.parse().ok()).unwrap_or(0),
            checksum: self.md5_checksum.unwrap_or_default(),
            properties: self.app_properties.unwrap_or_default(),
        }
    }
}

#[derive(Deserialize)]
struct FileList {
    #[serde(default)]
    files: Vec<ApiFile>,
}

#[derive(Deserialize, Default)]
struct ApiErrorEnvelope {
    #[serde(default)]
    error: ApiErrorBody,
}
#[derive(Deserialize, Default)]
struct ApiErrorBody {
    #[serde(default)]
    errors: Vec<ApiErrorItem>,
}
#[derive(Deserialize, Default)]
struct ApiErrorItem {
    #[serde(default)]
    reason: String,
}

/// Map an HTTP status + error body to the trait's [`DriveError`] cases (pure, so it
/// is unit-tested without the network). Reasons follow Drive v3's documented codes.
fn map_error(status: u16, body: &str) -> DriveError {
    let reason = serde_json::from_str::<ApiErrorEnvelope>(body)
        .ok()
        .and_then(|e| e.error.errors.into_iter().next().map(|i| i.reason))
        .unwrap_or_default();
    match status {
        401 => DriveError::TokenRevoked,
        403 => match reason.as_str() {
            "storageQuotaExceeded" => DriveError::QuotaFull,
            "rateLimitExceeded" | "userRateLimitExceeded" => DriveError::RateLimited,
            "insufficientFilePermissions" => DriveError::PermissionDenied,
            _ => DriveError::PermissionDenied,
        },
        404 => DriveError::NotFound,
        429 => DriveError::RateLimited,
        _ => DriveError::Io(format!("HTTP {status}: {}", body.chars().take(200).collect::<String>())),
    }
}

/// A Drive `q` filter for a named child of `parent` (optionally a folder), with the
/// value safely single-quote-escaped per Drive's query grammar.
fn child_query(parent: &str, name: &str, folder_only: bool) -> String {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('\'', "\\'");
    let mut q = format!("'{}' in parents and name = '{}' and trashed = false", esc(parent), esc(name));
    if folder_only {
        q.push_str(&format!(" and mimeType = '{FOLDER_MIME}'"));
    }
    q
}

/// Build a `multipart/related` upload body: a JSON metadata part followed by the raw
/// media bytes (pure, so it is unit-tested). Returns the body bytes; the caller sets
/// `Content-Type: multipart/related; boundary=<boundary>`.
fn multipart_related(metadata_json: &str, media: &[u8], boundary: &str) -> Vec<u8> {
    let mut body = Vec::with_capacity(metadata_json.len() + media.len() + 256);
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/json; charset=UTF-8\r\n\r\n");
    body.extend_from_slice(metadata_json.as_bytes());
    body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(media);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    body
}

impl GoogleDrive {
    /// Await `fut` on the client's own runtime. Safe from any non-async (blocking)
    /// thread; must not be called from inside another Tokio runtime.
    fn block<F: std::future::Future>(&self, fut: F) -> F::Output {
        self.rt.block_on(fut)
    }

    /// Read the response body, returning it on success or the mapped error otherwise.
    async fn ok_text(resp: reqwest::Response) -> DriveResult<String> {
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| DriveError::Io(e.to_string()))?;
        if (200..300).contains(&status) {
            Ok(body)
        } else {
            Err(map_error(status, &body))
        }
    }

    async fn ok_json<T: DeserializeOwned>(resp: reqwest::Response) -> DriveResult<T> {
        let body = Self::ok_text(resp).await?;
        serde_json::from_str(&body).map_err(|e| DriveError::Io(e.to_string()))
    }

    async fn find_child_async(&self, parent: &str, name: &str, folder_only: bool) -> DriveResult<Option<ApiFile>> {
        let q = child_query(parent, name, folder_only);
        let fields = format!("files({FIELDS})");
        let resp = self
            .http
            .get(API)
            .bearer_auth(&self.access_token)
            .query(&[
                ("q", q.as_str()),
                ("fields", fields.as_str()),
                ("spaces", "drive"),
                ("pageSize", "1"),
            ])
            .send()
            .await
            .map_err(|e| DriveError::Io(e.to_string()))?;
        let list: FileList = Self::ok_json(resp).await?;
        Ok(list.files.into_iter().next())
    }
}

impl DriveApi for GoogleDrive {
    fn list(&self, folder_id: &str) -> DriveResult<Vec<DriveFile>> {
        self.block(async {
            let q = format!("'{}' in parents and trashed = false", folder_id.replace('\'', "\\'"));
            let fields = format!("files({FIELDS})");
            let resp = self
                .http
                .get(API)
                .bearer_auth(&self.access_token)
                .query(&[
                    ("q", q.as_str()),
                    ("fields", fields.as_str()),
                    ("spaces", "drive"),
                    ("pageSize", "1000"),
                ])
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let list: FileList = Self::ok_json(resp).await?;
            Ok(list.files.into_iter().map(ApiFile::into_drive).collect())
        })
    }

    fn metadata(&self, file_id: &str) -> DriveResult<DriveFile> {
        self.block(async {
            let resp = self
                .http
                .get(format!("{API}/{file_id}"))
                .bearer_auth(&self.access_token)
                .query(&[("fields", FIELDS)])
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let f: ApiFile = Self::ok_json(resp).await?;
            Ok(f.into_drive())
        })
    }

    fn download(&self, file_id: &str) -> DriveResult<Vec<u8>> {
        self.block(async {
            let resp = self
                .http
                .get(format!("{API}/{file_id}"))
                .bearer_auth(&self.access_token)
                .query(&[("alt", "media")])
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let status = resp.status().as_u16();
            if !(200..300).contains(&status) {
                let body = resp.text().await.unwrap_or_default();
                return Err(map_error(status, &body));
            }
            let bytes = resp.bytes().await.map_err(|e| DriveError::Io(e.to_string()))?;
            Ok(bytes.to_vec())
        })
    }

    fn create(
        &self,
        parent: &str,
        name: &str,
        bytes: &[u8],
        properties: &BTreeMap<String, String>,
    ) -> DriveResult<DriveFile> {
        self.block(async {
            let meta = serde_json::json!({
                "name": name,
                "parents": [parent],
                "appProperties": properties,
            })
            .to_string();
            let boundary = format!("vidyabk{:x}", crc(bytes) ^ crc(name.as_bytes()));
            let body = multipart_related(&meta, bytes, &boundary);
            let resp = self
                .http
                .post(UPLOAD)
                .bearer_auth(&self.access_token)
                .query(&[("uploadType", "multipart"), ("fields", FIELDS)])
                .header(reqwest::header::CONTENT_TYPE, format!("multipart/related; boundary={boundary}"))
                .body(body)
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let f: ApiFile = Self::ok_json(resp).await?;
            Ok(f.into_drive())
        })
    }

    fn rename(&self, file_id: &str, new_name: &str) -> DriveResult<DriveFile> {
        self.block(async {
            let resp = self
                .http
                .patch(format!("{API}/{file_id}"))
                .bearer_auth(&self.access_token)
                .query(&[("fields", FIELDS)])
                .json(&serde_json::json!({ "name": new_name }))
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let f: ApiFile = Self::ok_json(resp).await?;
            Ok(f.into_drive())
        })
    }

    fn move_to(&self, file_id: &str, new_parent: &str) -> DriveResult<DriveFile> {
        self.block(async {
            // Need the current parent to remove it.
            let cur: ApiFile = {
                let resp = self
                    .http
                    .get(format!("{API}/{file_id}"))
                    .bearer_auth(&self.access_token)
                    .query(&[("fields", "id,parents")])
                    .send()
                    .await
                    .map_err(|e| DriveError::Io(e.to_string()))?;
                Self::ok_json(resp).await?
            };
            let remove = cur.parents.unwrap_or_default().join(",");
            let resp = self
                .http
                .patch(format!("{API}/{file_id}"))
                .bearer_auth(&self.access_token)
                .query(&[
                    ("addParents", new_parent),
                    ("removeParents", remove.as_str()),
                    ("fields", FIELDS),
                ])
                .json(&serde_json::json!({}))
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let f: ApiFile = Self::ok_json(resp).await?;
            Ok(f.into_drive())
        })
    }

    fn delete(&self, file_id: &str) -> DriveResult<()> {
        self.block(async {
            let resp = self
                .http
                .delete(format!("{API}/{file_id}"))
                .bearer_auth(&self.access_token)
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            Self::ok_text(resp).await.map(|_| ())
        })
    }

    fn ensure_folder(&self, parent: &str, name: &str) -> DriveResult<String> {
        self.block(async {
            if let Some(existing) = self.find_child_async(parent, name, true).await? {
                return Ok(existing.id);
            }
            let resp = self
                .http
                .post(API)
                .bearer_auth(&self.access_token)
                .query(&[("fields", "id")])
                .json(&serde_json::json!({
                    "name": name,
                    "mimeType": FOLDER_MIME,
                    "parents": [parent],
                }))
                .send()
                .await
                .map_err(|e| DriveError::Io(e.to_string()))?;
            let f: ApiFile = Self::ok_json(resp).await?;
            Ok(f.id)
        })
    }
}

/// A tiny non-cryptographic hash, only to vary the multipart boundary per upload so
/// it cannot appear in the payload. (Not security-sensitive.)
fn crc(bytes: &[u8]) -> u32 {
    let mut h: u32 = 2166136261;
    for &b in bytes {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_http_errors_to_drive_cases() {
        assert_eq!(map_error(401, "{}"), DriveError::TokenRevoked);
        assert_eq!(map_error(404, "{}"), DriveError::NotFound);
        assert_eq!(map_error(429, "{}"), DriveError::RateLimited);
        let quota = r#"{"error":{"errors":[{"reason":"storageQuotaExceeded"}]}}"#;
        assert_eq!(map_error(403, quota), DriveError::QuotaFull);
        let rate = r#"{"error":{"errors":[{"reason":"userRateLimitExceeded"}]}}"#;
        assert_eq!(map_error(403, rate), DriveError::RateLimited);
        let perm = r#"{"error":{"errors":[{"reason":"insufficientFilePermissions"}]}}"#;
        assert_eq!(map_error(403, perm), DriveError::PermissionDenied);
        assert!(matches!(map_error(500, "boom"), DriveError::Io(_)));
    }

    #[test]
    fn api_file_maps_to_drive_file() {
        let json = r#"{"id":"f1","name":"x.vbak","mimeType":"application/octet-stream","size":"1234","md5Checksum":"abc","parents":["p1"],"appProperties":{"k":"v"}}"#;
        let f: ApiFile = serde_json::from_str(json).unwrap();
        let d = f.into_drive();
        assert_eq!(d.id, "f1");
        assert_eq!(d.name, "x.vbak");
        assert_eq!(d.parent.as_deref(), Some("p1"));
        assert!(!d.is_folder);
        assert_eq!(d.size, 1234);
        assert_eq!(d.checksum, "abc");
        assert_eq!(d.properties.get("k").map(String::as_str), Some("v"));
    }

    #[test]
    fn folder_mime_is_detected() {
        let json = format!(r#"{{"id":"d1","name":"Vidya","mimeType":"{FOLDER_MIME}"}}"#);
        let f: ApiFile = serde_json::from_str(&json).unwrap();
        assert!(f.into_drive().is_folder);
    }

    #[test]
    fn child_query_escapes_quotes() {
        let q = child_query("root", "O'Brien", true);
        assert!(q.contains("'root' in parents"));
        assert!(q.contains("name = 'O\\'Brien'"));
        assert!(q.contains("trashed = false"));
        assert!(q.contains(FOLDER_MIME));
    }

    #[test]
    fn multipart_body_has_both_parts() {
        let body = multipart_related(r#"{"name":"x"}"#, &[1, 2, 3], "BND");
        let s = String::from_utf8_lossy(&body);
        assert!(s.starts_with("--BND\r\n"));
        assert!(s.contains("application/json"));
        assert!(s.contains(r#"{"name":"x"}"#));
        assert!(s.contains("application/octet-stream"));
        assert!(s.trim_end().ends_with("--BND--"));
        // the raw media bytes are present between the second header and the closing boundary
        assert!(body.windows(3).any(|w| w == [1, 2, 3]));
    }
}
