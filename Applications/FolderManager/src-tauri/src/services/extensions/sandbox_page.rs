// The page the extension iframes load: a scheme of its own, so it carries its own CSP instead of the app's.
use tauri::http::Response;

pub const SCHEME: &str = "extsandbox";

// Inline scripts are allowed here only because the iframe is sandboxed and cannot reach the network.
const CSP: &str = "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:";

const LOADER: &str = r#"<!doctype html><meta charset="utf-8"><script>addEventListener("message",(e)=>{if(e.source!==parent||!e.data||e.data.kind!=="doc"||typeof e.data.html!=="string")return;document.open();document.write(e.data.html);document.close()},{once:true})</script>"#;

pub fn response() -> Response<Vec<u8>> {
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Content-Security-Policy", CSP)
        .body(LOADER.as_bytes().to_vec())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loader_page_carries_its_own_csp_and_only_accepts_the_parent() {
        let r = response();
        assert_eq!(r.headers()["Content-Security-Policy"], CSP);
        let body = String::from_utf8(r.into_body()).unwrap();
        assert!(body.contains("e.source!==parent"));
    }
}
