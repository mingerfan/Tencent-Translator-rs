use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use reqwest::blocking::Client;
use serde_json::json;
use ring::hmac;

use super::error::{TranslatorError, TranslationError};
use super::config::OutputFormat;

/// Trait for implementing translation backends
pub trait TranslationBackend: Send + Sync {
    /// Translate the given text from the source language to the target language
    fn translate(&self, text: &str, from: &str, to: &str) -> anyhow::Result<String>;

    /// Get the name of this backend
    fn name(&self) -> &'static str;

    /// Check if this backend is available for use (e.g. has valid credentials)
    fn is_available(&self) -> bool;

    /// Get the output format
    fn output_format(&self) -> &OutputFormat;

    /// Format the translated text according to the output format
    fn format_output(&self, text: String) -> String {
        match self.output_format() {
            OutputFormat::Markdown => super::config::markdown_to_html(&text),
            OutputFormat::PlainText => text,
        }
    }
}

/// Common configuration for translation backends
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum BackendConfig {
    #[serde(rename = "tencent")]
    Tencent {
        secret_id: String,
        secret_key: String,
        #[serde(default)]
        format: OutputFormat,
    },
    #[serde(rename = "openai_compatible")]
    OpenAICompatible {
        name: String,
        api_key: String,
        model: String,
        api_base: String,
        #[serde(default)]
        format: OutputFormat,
    },
}

impl BackendConfig {
    pub fn as_backend(&self) -> Box<dyn TranslationBackend> {
        match self {
            BackendConfig::Tencent { secret_id, secret_key, format } => {
                Box::new(TencentBackend::new(secret_id.clone(), secret_key.clone(), format.clone()))
            }
            BackendConfig::OpenAICompatible { name, api_key, model, api_base, format } => {
                Box::new(OpenAICompatibleBackend::new(
                    name.clone(),
                    api_key.clone(),
                    model.clone(),
                    api_base.clone(),
                    format.clone(),
                ))
            }
        }
    }
}

/// Tencent Cloud translation backend implementation
pub struct TencentBackend {
    secret_id: String,
    secret_key: String,
    format: OutputFormat,
}

impl TencentBackend {
    pub fn new(secret_id: String, secret_key: String, format: OutputFormat) -> Self {
        Self {
            secret_id,
            secret_key,
            format,
        }
    }

    fn sign(&self, key: &[u8], msg: &[u8]) -> Vec<u8> {
        let s_key = hmac::Key::new(hmac::HMAC_SHA256, key);
        hmac::sign(&s_key, msg).as_ref().to_vec()
    }
}

impl TranslationBackend for TencentBackend {
    fn output_format(&self) -> &OutputFormat {
        &self.format
    }

    fn translate(&self, text: &str, from: &str, to: &str) -> anyhow::Result<String> {
        let service = "tmt";
        let host = "tmt.tencentcloudapi.com";
        let region = "ap-guangzhou";
        let version = "2018-03-21";
        let action = "TextTranslate";
        let payload = json!({
            "SourceText": text,
            "Source": from,
            "Target": to,
            "ProjectId": 0
        });
        let endpoint = "https://tmt.tencentcloudapi.com";
        let algorithm = "TC3-HMAC-SHA256";

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| TranslationError::ServiceError(e.to_string()))?
            .as_secs();

        let date = chrono::DateTime::from_timestamp(timestamp as i64, 0)
            .ok_or_else(|| TranslationError::ServiceError("Failed to get date".to_string()))?
            .format("%Y-%m-%d")
            .to_string();

        // Step 1: Create Canonical Request
        let http_request_method = "POST";
        let canonical_uri = "/";
        let canonical_querystring = "";
        let ct = "application/json; charset=utf-8";
        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-tc-action:{}\n",
            ct,
            host,
            action.to_lowercase()
        );
        let signed_headers = "content-type;host;x-tc-action";
        let hashed_request_payload =
            ring::digest::digest(&ring::digest::SHA256, payload.to_string().as_bytes());
        let payload_hash = hex::encode(hashed_request_payload);
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            http_request_method,
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            payload_hash,
        );

        // Step 2: Create String to Sign
        let credential_scope = format!("{}/{}/tc3_request", date, service);
        let hashed_canonical_request =
            ring::digest::digest(&ring::digest::SHA256, canonical_request.as_bytes());
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            timestamp,
            credential_scope,
            hex::encode(hashed_canonical_request)
        );

        // Step 3: Calculate Signature
        let secret_date = self.sign(format!("TC3{}", self.secret_key).as_bytes(), date.as_bytes());
        let secret_service = self.sign(&secret_date, service.as_bytes());
        let secret_signing = self.sign(&secret_service, b"tc3_request");
        let signature = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, &secret_signing),
            string_to_sign.as_bytes(),
        );

        // Step 4: Create Authorization
        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm,
            self.secret_id,
            credential_scope,
            signed_headers,
            hex::encode(signature)
        );

        // Step 5: Send Request
        let client = Client::new();
        let mut headers = HashMap::new();
        headers.insert("Authorization", authorization);
        headers.insert("Content-Type", ct.to_string());
        headers.insert("Host", host.to_string());
        headers.insert("X-TC-Action", action.to_string());
        headers.insert("X-TC-Timestamp", timestamp.to_string());
        headers.insert("X-TC-Version", version.to_string());
        headers.insert("X-TC-Region", region.to_string());

        let response = client
            .post(endpoint)
            .headers(reqwest::header::HeaderMap::from_iter(
                headers
                    .iter()
                    .map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())),
            ))
            .json(&payload)
            .send()
            .map_err(|e| TranslationError::NetworkError(e.to_string()))?;

        let status = response.status();
        let res = response
            .json::<serde_json::Value>()
            .map_err(|e| TranslationError::InvalidResponse(e.to_string()))?;

        if !status.is_success() {
            return Err(TranslationError::ServiceError(format!("API error: {:?}", res)).into());
        }

        if let Some(text) = res["Response"]["TargetText"].as_str() {
            Ok(self.format_output(text.to_string()))
        } else {
            Err(TranslationError::InvalidResponse(format!("Invalid response format: {:?}", res)).into())
        }
    }

    fn name(&self) -> &'static str {
        "tencent"
    }

    fn is_available(&self) -> bool {
        !self.secret_id.is_empty() && !self.secret_key.is_empty()
    }
}

/// OpenAI-compatible translation backend implementation
pub struct OpenAICompatibleBackend {
    name: String,
    api_key: String,
    model: String,
    api_base: String,
    format: OutputFormat,
}

impl OpenAICompatibleBackend {
    pub fn new(name: String, api_key: String, model: String, api_base: String, format: OutputFormat) -> Self {
        Self {
            name,
            api_key,
            model,
            api_base,
            format,
        }
    }
}

impl TranslationBackend for OpenAICompatibleBackend {
    fn output_format(&self) -> &OutputFormat {
        &self.format
    }

    fn translate(&self, text: &str, from: &str, to: &str) -> anyhow::Result<String> {
        let prompt = match super::Config::load()?.get_prompt("translation") {
            Some(template) => template
                .replace("{text}", text)
                .replace("{source}", from)
                .replace("{target}", to),
            None => return Err(TranslatorError::ConfigError("Translation prompt not found".to_string()).into()),
        };

        let system_prompt = match self.format {
            OutputFormat::Markdown => "You are a professional translator. Format your translation using markdown \
                for better readability. Use markdown features like **bold**, *italic*, lists, headers when appropriate.",
            OutputFormat::PlainText => "You are a professional translator.",
        };

        let client = Client::new();
        println!("posting to {}", format!("{}/chat/completions", self.api_base));
        let response = client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "system",
                        "content": system_prompt
                    },
                    {
                        "role": "user",
                        "content": prompt
                    }
                ],
                "temperature": 0.3
            }))
            .send()
            .map_err(|e| TranslationError::NetworkError(e.to_string()))?;

        let status = response.status();
        let res = response
            .json::<serde_json::Value>()
            .map_err(|e| TranslationError::InvalidResponse(e.to_string()))?;

        if !status.is_success() {
            if status.as_u16() == 429 {
                return Err(TranslationError::RateLimitExceeded.into());
            }
            return Err(TranslationError::ServiceError(format!("API error: {:?}", res)).into());
        }

        if let Some(translation) = res["choices"][0]["message"]["content"].as_str() {
            Ok(self.format_output(translation.trim().to_string()))
        } else {
            Err(TranslationError::InvalidResponse(format!("Invalid response format: {:?}", res)).into())
        }
    }

    fn name(&self) -> &'static str {
        Box::leak(self.name.clone().into_boxed_str())
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty() && !self.model.is_empty() && !self.api_base.is_empty()
    }
}
