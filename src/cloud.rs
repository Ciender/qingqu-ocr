//! Official Tencent TC3 and Alibaba ACS3 signing. No credential-bearing URLs.
use crate::config::Credentials;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ptr::{null, null_mut},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::SYSTEMTIME, Security::Cryptography::*, System::SystemInformation::GetSystemTime,
};

fn digest(key: Option<&[u8]>, data: &[u8]) -> Result<[u8; 32], String> {
    unsafe {
        let mut alg = null_mut();
        let name: Vec<u16> = "SHA256\0".encode_utf16().collect();
        if BCryptOpenAlgorithmProvider(
            &mut alg,
            name.as_ptr(),
            null(),
            if key.is_some() {
                BCRYPT_ALG_HANDLE_HMAC_FLAG
            } else {
                0
            },
        ) < 0
        {
            return Err("Windows 无法初始化 SHA-256。".into());
        }
        let mut out = [0; 32];
        let key = key.unwrap_or_default();
        let status = BCryptHash(
            alg,
            if key.is_empty() { null() } else { key.as_ptr() },
            key.len() as u32,
            data.as_ptr(),
            data.len() as u32,
            out.as_mut_ptr(),
            32,
        );
        BCryptCloseAlgorithmProvider(alg, 0);
        if status < 0 {
            Err("Windows 请求签名失败。".into())
        } else {
            Ok(out)
        }
    }
}
fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}
fn sha(data: &[u8]) -> Result<String, String> {
    digest(None, data).map(|b| hex(&b))
}
fn hmac(key: &[u8], data: &str) -> Result<[u8; 32], String> {
    digest(Some(key), data.as_bytes())
}
fn utc() -> (String, String, u64) {
    let mut t = SYSTEMTIME::default();
    unsafe {
        GetSystemTime(&mut t);
    }
    (
        format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay),
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
        ),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
}
pub fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(12))
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法初始化安全网络连接。".into())
}
fn response(response: reqwest::blocking::Response, vendor: &str) -> Result<Value, String> {
    let status = response.status();
    let body: Value = response
        .json()
        .map_err(|_| format!("{vendor} 返回了无法解析的响应。"))?;
    if !status.is_success() {
        return Err(format!(
            "{vendor} 返回 HTTP {}，请检查凭据、产品权限及额度。",
            status.as_u16()
        ));
    }
    Ok(body)
}
fn tc_auth(
    id: &str,
    secret: &str,
    service: &str,
    action: &str,
    payload: &[u8],
    date: &str,
    timestamp: u64,
) -> Result<String, String> {
    let headers = format!("content-type:application/json; charset=utf-8\nhost:{service}.tencentcloudapi.com\nx-tc-action:{}\n", action.to_ascii_lowercase());
    let signed = "content-type;host;x-tc-action";
    let canonical = format!("POST\n/\n\n{headers}\n{signed}\n{}", sha(payload)?);
    let scope = format!("{date}/{service}/tc3_request");
    let to_sign = format!(
        "TC3-HMAC-SHA256\n{timestamp}\n{scope}\n{}",
        sha(canonical.as_bytes())?
    );
    let date_key = hmac(format!("TC3{secret}").as_bytes(), date)?;
    let service_key = hmac(&date_key, service)?;
    let signing = hmac(&service_key, "tc3_request")?;
    Ok(format!(
        "TC3-HMAC-SHA256 Credential={id}/{scope}, SignedHeaders={signed}, Signature={}",
        hex(&hmac(&signing, &to_sign)?)
    ))
}
pub fn tencent(
    service: &str,
    action: &str,
    region: &str,
    body: &Value,
    keys: &Credentials,
) -> Result<Value, String> {
    if keys.tencent_secret_id.trim().is_empty() || keys.tencent_secret_key.trim().is_empty() {
        return Err("请填写腾讯云 SecretId 和 SecretKey。".into());
    }
    if !matches!(service, "ocr" | "tmt") {
        return Err("不支持的腾讯云服务。".into());
    }
    let payload = serde_json::to_vec(body).map_err(|_| "请求序列化失败。")?;
    let (date, _, timestamp) = utc();
    let auth = tc_auth(
        keys.tencent_secret_id.trim(),
        keys.tencent_secret_key.trim(),
        service,
        action,
        &payload,
        &date,
        timestamp,
    )?;
    let result = client()?
        .post(format!("https://{service}.tencentcloudapi.com/"))
        .header("content-type", "application/json; charset=utf-8")
        .header("X-TC-Action", action)
        .header(
            "X-TC-Version",
            if service == "ocr" {
                "2018-11-19"
            } else {
                "2018-03-21"
            },
        )
        .header("X-TC-Timestamp", timestamp.to_string())
        .header("X-TC-Region", region.trim())
        .header("Authorization", auth)
        .body(payload)
        .send()
        .map_err(|_| "腾讯云连接失败或超时。")?;
    let result = response(result, "腾讯云")?;
    let data = result.get("Response").ok_or("腾讯云响应缺少 Response。")?;
    if let Some(error) = data.get("Error") {
        let code = error
            .get("Code")
            .and_then(Value::as_str)
            .unwrap_or("Unknown");
        // Never display raw remote messages: they may echo a credential or text.
        return Err(format!("腾讯云错误 {code}；请检查密钥、权限、地域和额度。"));
    }
    Ok(data.clone())
}
fn ali_auth(
    id: &str,
    secret: &str,
    path: &str,
    body: &[u8],
    headers: &BTreeMap<String, String>,
    query: &str,
) -> Result<String, String> {
    let names = headers.keys().cloned().collect::<Vec<_>>().join(";");
    let normalized = headers
        .iter()
        .map(|(k, v)| format!("{k}:{}\n", v.trim()))
        .collect::<String>();
    let request = format!(
        "POST\n{path}\n{query}\n{normalized}\n{names}\n{}",
        sha(body)?
    );
    let sign = format!("ACS3-HMAC-SHA256\n{}", sha(request.as_bytes())?);
    Ok(format!(
        "ACS3-HMAC-SHA256 Credential={id},SignedHeaders={names},Signature={}",
        hex(&hmac(secret.as_bytes(), &sign)?)
    ))
}
pub fn aliyun(
    action: &str,
    region: &str,
    body: Vec<u8>,
    translation: bool,
    keys: &Credentials,
) -> Result<Value, String> {
    aliyun_query(action, region, body, translation, keys, &[])
}
pub fn aliyun_query(
    action: &str,
    region: &str,
    body: Vec<u8>,
    translation: bool,
    keys: &Credentials,
    parameters: &[(&str, &str)],
) -> Result<Value, String> {
    if keys.aliyun_access_key_id.trim().is_empty()
        || keys.aliyun_access_key_secret.trim().is_empty()
    {
        return Err("请填写阿里云 AccessKey ID 和 AccessKey Secret。".into());
    }
    if !matches!(region, "cn-hangzhou" | "ap-southeast-1") {
        return Err("阿里云 OCR 地域请选择杭州或新加坡。".into());
    }
    let host = if translation {
        "mt.cn-hangzhou.aliyuncs.com".into()
    } else {
        format!("ocr-api.{region}.aliyuncs.com")
    };
    // ACS3 RPC protocol, as in Alibaba's current signing example. The older
    // /api/translate/web/general REST path uses a different request contract.
    let path = "/";
    let (_, date, _) = utc();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .to_string();
    let headers = BTreeMap::from([
        ("host".into(), host),
        (
            "content-type".into(),
            if translation {
                "application/x-www-form-urlencoded"
            } else {
                "application/octet-stream"
            }
            .into(),
        ),
        ("x-acs-action".into(), action.into()),
        (
            "x-acs-version".into(),
            if translation {
                "2018-10-12"
            } else {
                "2021-07-07"
            }
            .into(),
        ),
        ("x-acs-date".into(), date),
        ("x-acs-signature-nonce".into(), nonce),
        ("x-acs-content-sha256".into(), sha(&body)?),
    ]);
    let auth = ali_auth(
        keys.aliyun_access_key_id.trim(),
        keys.aliyun_access_key_secret.trim(),
        path,
        &body,
        &headers,
        &query_string(parameters),
    )?;
    let query = query_string(parameters);
    let suffix = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    let mut req = client()?
        .post(format!("https://{}{path}{suffix}", headers["host"]))
        .header("Authorization", auth);
    for (name, value) in headers {
        req = req.header(&name, &value);
    }
    let result = response(
        req.body(body)
            .send()
            .map_err(|_| "阿里云连接失败或超时。")?,
        "阿里云",
    )?;
    if let Some(code) = result.get("Code").and_then(Value::as_str) {
        if code != "200" && code != "Success" {
            return Err(format!(
                "阿里云错误 {code}；请检查产品开通、密钥权限和额度。"
            ));
        }
    }
    if result
        .get("Code")
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 200)
    {
        return Err("阿里云返回业务错误，请检查权限和额度。".into());
    }
    Ok(result)
}
fn query_string(parameters: &[(&str, &str)]) -> String {
    let encode = |s: &str| {
        s.bytes()
            .map(|b| {
                if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                    (b as char).to_string()
                } else {
                    format!("%{b:02X}")
                }
            })
            .collect::<String>()
    };
    let mut parts = parameters
        .iter()
        .map(|(k, v)| (encode(k), encode(v)))
        .collect::<Vec<_>>();
    parts.sort();
    parts
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}
