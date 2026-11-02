pub const API_AUDIENCE: &str = "https://api.storyteller.ai";
pub const CONNECTOR_AUDIENCE: &str = "https://connect.getartcraft.com/mcp";

const MAX_SCOPE_LEN: usize = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
  Generate,
  LibraryRead,
  LibraryWrite,
  Connector,
}

impl Scope {
  pub fn as_str(self) -> &'static str {
    match self {
      Self::Generate => "generate",
      Self::LibraryRead => "library:read",
      Self::LibraryWrite => "library:write",
      Self::Connector => "connector",
    }
  }

  pub fn description(self) -> &'static str {
    match self {
      Self::Generate => "Generate media. This spends credits on your account.",
      Self::LibraryRead => "Read files in your library.",
      Self::LibraryWrite => "Add and change files in your library.",
      Self::Connector => "Call the hosted connector on your behalf.",
    }
  }
}

/// Canonical scope string for `audience`, or `None` when the combination is not allowed.
/// `generation` is accepted as an alias of `generate`.
pub fn canonicalize_scopes(raw: &str, audience: &str) -> Option<String> {
  if raw.is_empty() || raw.len() > MAX_SCOPE_LEN {
    return None;
  }
  if raw.chars().any(|c| c.is_control() || !c.is_ascii()) {
    return None;
  }

  let mut scopes = Vec::new();
  for part in raw.split_whitespace() {
    let scope = match part {
      "generate" | "generation" => Scope::Generate,
      "library:read" => Scope::LibraryRead,
      "library:write" => Scope::LibraryWrite,
      "connector" => Scope::Connector,
      _ => return None,
    };
    if !scopes.contains(&scope) {
      scopes.push(scope);
    }
  }
  if scopes.is_empty() {
    return None;
  }

  let connector_only = scopes.len() == 1 && scopes[0] == Scope::Connector;
  let includes_connector = scopes.contains(&Scope::Connector);
  if audience == CONNECTOR_AUDIENCE {
    if !connector_only {
      return None;
    }
  } else if audience == API_AUDIENCE {
    if includes_connector {
      return None;
    }
  } else {
    return None;
  }

  scopes.sort_by_key(|scope| scope_order(*scope));
  Some(scopes.iter().map(|scope| scope.as_str()).collect::<Vec<_>>().join(" "))
}

pub fn scope_granted(stored: &str, required: Scope) -> bool {
  stored.split(' ').any(|part| part == required.as_str())
}

pub fn scopes_from_canonical(stored: &str) -> Vec<Scope> {
  stored
    .split(' ')
    .filter_map(|part| match part {
      "generate" => Some(Scope::Generate),
      "library:read" => Some(Scope::LibraryRead),
      "library:write" => Some(Scope::LibraryWrite),
      "connector" => Some(Scope::Connector),
      _ => None,
    })
    .collect()
}

fn scope_order(scope: Scope) -> u8 {
  match scope {
    Scope::Generate => 0,
    Scope::LibraryRead => 1,
    Scope::LibraryWrite => 2,
    Scope::Connector => 3,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generation_is_an_alias_of_generate() {
    assert_eq!(canonicalize_scopes("generation library:read", API_AUDIENCE).as_deref(), Some("generate library:read"),);
  }

  #[test]
  fn unknown_scope_is_rejected() {
    assert_eq!(canonicalize_scopes("generate admin", API_AUDIENCE), None);
  }

  #[test]
  fn connector_audience_is_only_the_connector_scope() {
    assert_eq!(canonicalize_scopes("connector", CONNECTOR_AUDIENCE).as_deref(), Some("connector"),);
    assert_eq!(canonicalize_scopes("connector generate", CONNECTOR_AUDIENCE), None);
    assert_eq!(canonicalize_scopes("generate", CONNECTOR_AUDIENCE), None);
  }

  #[test]
  fn api_audience_rejects_the_connector_scope() {
    assert_eq!(canonicalize_scopes("connector", API_AUDIENCE), None);
    assert_eq!(canonicalize_scopes("generate connector", API_AUDIENCE), None);
    assert_eq!(canonicalize_scopes("generate", API_AUDIENCE).as_deref(), Some("generate"));
  }
}
