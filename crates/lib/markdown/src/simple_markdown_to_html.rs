use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};

/// Only these URL schemes (or scheme-less relative/anchor URLs) may appear in generated links.
fn is_safe_url(url: &str) -> bool {
  // Browsers ignore ASCII whitespace/control characters inside the scheme.
  let compact: String = url.chars().filter(|c| !c.is_ascii_control() && !c.is_whitespace()).collect();
  let lower = compact.to_ascii_lowercase();
  match lower.split_once(':') {
    Some((scheme, _)) if !scheme.contains(['/', '?', '#']) => matches!(scheme, "http" | "https" | "mailto"),
    _ => true,
  }
}

fn sanitize_event(event: Event<'_>) -> Event<'_> {
  match event {
    Event::Start(Tag::Link { link_type, dest_url, title, id }) if !is_safe_url(&dest_url) => {
      Event::Start(Tag::Link { link_type, dest_url: CowStr::Borrowed("#"), title, id })
    }
    Event::Start(Tag::Image { link_type, dest_url, title, id }) if !is_safe_url(&dest_url) => {
      Event::Start(Tag::Image { link_type, dest_url: CowStr::Borrowed("#"), title, id })
    }
    other => other,
  }
}

pub fn simple_markdown_to_html(markdown_input: &str) -> String {
  // NB: CommonMark allows for HTML, including <script>! WTF!
  // Let's prevent that nonsense.
  // https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html
  let markdown_input = markdown_input
      .replace("&", "&amp;")
      .replace("<", "&lt;")
      .replace(">", "&gt;")
      .replace("\"", "&quot;")
      .replace("'", "&#x27;"); // NB: &apos; is not in the HTML spec

  let mut options = Options::empty();
  options.insert(Options::ENABLE_STRIKETHROUGH);

  let parser = Parser::new_ext(&markdown_input, options);

  // NB: The buffer size can expand, but the math is a sizing heuristic I found in one of the
  // examples that may prevent buffer resizing.
  let mut html_output: String = String::with_capacity(markdown_input.len() * 3 / 2);
  html::push_html(&mut html_output, parser.map(sanitize_event));

  html_output
}

#[cfg(test)]
mod tests {
  use crate::simple_markdown_to_html::simple_markdown_to_html;

  #[test]
  fn handles_markdown() {
    assert_eq!(&simple_markdown_to_html("*italics text*"),
      "<p><em>italics text</em></p>\n");

    assert_eq!(&simple_markdown_to_html("**bold text**"),
      "<p><strong>bold text</strong></p>\n");
  }

  #[test]
  fn do_not_allow_html_entities() {
    assert_eq!(&simple_markdown_to_html("this > that"),
      "<p>this &gt; that</p>\n");

    assert_eq!(&simple_markdown_to_html("&"),
      "<p>&amp;</p>\n");

    // NB: We doubly entity decode because pulldown_cmark doesn't do enough!
    assert_eq!(&simple_markdown_to_html("&gt;"),
      "<p>&amp;gt;</p>\n");
  }

  #[test]
  fn do_not_allow_script_urls() {
    for bad in ["[x](javascript:alert(1))", "[x](JaVaScRiPt:alert(1))", "![x](data:text/html;base64,AAAA)", "[x](vbscript:y)"] {
      let html = simple_markdown_to_html(bad);
      assert!(html.contains("=\"#\""), "{} -> {}", bad, html);
    }
  }

  #[test]
  fn keep_normal_links() {
    for good in ["https://a.example/x?y=1:2", "http://a.example", "mailto:a@b.example", "/relative:path", "#anchor", "page.html"] {
      let html = simple_markdown_to_html(&format!("[x]({good})"));
      assert!(html.contains(&format!("href=\"{good}\"")), "{} -> {}", good, html);
    }
  }

  #[test]
  fn do_not_allow_html_tags() {
    assert_eq!(&simple_markdown_to_html("<script>alert();</script>"),
      "<p>&lt;script&gt;alert();&lt;/script&gt;</p>\n");
  }
}
