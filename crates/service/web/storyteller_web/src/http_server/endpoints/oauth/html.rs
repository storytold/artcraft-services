/// Escape text that is interpolated into the consent and grants pages.
pub fn html_escape(value: &str) -> String {
  let mut out = String::with_capacity(value.len());
  for character in value.chars() {
    match character {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      _ => out.push(character),
    }
  }
  out
}

/// Error page. `message` is escaped.
pub fn error_page(message: &str) -> String {
  page("Authorization request", &format!("<p>{}</p>", html_escape(message)))
}

/// Page shown when the authorization or grants request has no website session.
pub fn sign_in_page() -> String {
  page("Sign in required", "<p>Sign in on the website, then open this authorization link again.</p>")
}

/// Consent form. `client_name` is escaped. `scope_items` and `fields` are already markup.
pub fn consent_page(client_name: &str, scope_items: &str, fields: &str) -> String {
  let body = format!(
    "<p><strong>{}</strong> is asking for access to your account.</p><ul>{}</ul>\
     <form method=\"post\" action=\"/v1/oauth/authorize\">{}{}\
     <button type=\"submit\" name=\"decision\" value=\"approve\">Allow</button>\
     <button type=\"submit\" name=\"decision\" value=\"deny\">Deny</button></form>",
    html_escape(client_name),
    scope_items,
    fields,
    ""
  );
  page("Allow access", &body)
}

/// Connected-apps page. An empty list says no apps have access. `rows` is already markup.
pub fn grants_page(rows: &str) -> String {
  let body = if rows.is_empty() { "<p>No apps currently have access.</p>".to_string() } else { format!("<ul>{rows}</ul>") };
  page("Connected apps", &body)
}

/// Hidden form field. Both the name and the value are escaped.
pub fn hidden_input(name: &str, value: &str) -> String {
  format!("<input type=\"hidden\" name=\"{}\" value=\"{}\">", html_escape(name), html_escape(value),)
}

/// Shared document shell. `title` is escaped. `body` is already markup.
fn page(title: &str, body: &str) -> String {
  format!("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1>{}</body></html>", html_escape(title), html_escape(title), body,)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn markup_in_a_client_name_is_escaped() {
    let page = consent_page("<script>", "", "");
    assert!(page.contains("&lt;script&gt;"));
    assert!(!page.contains("<script>"));
  }
}
