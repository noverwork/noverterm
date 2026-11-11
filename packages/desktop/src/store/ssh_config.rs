//! Imports hosts from the user's OpenSSH client config (`~/.ssh/config`).
//!
//! Supports `Host` blocks (with `*`, `?` and `!` patterns), `Include`, and the
//! `HostName`, `User`, `Port`, `IdentityFile` and `ProxyJump` keywords. Like
//! ssh, the first value obtained for a keyword wins, so `Host *` defaults at
//! the end of the file fill in whatever the specific block left unset.
// ponytail: `Match` blocks and `%h`-style tokens are ignored; add them if users report missing hosts.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use diesel::prelude::*;
use orm::models::{NewSshHost, NewSshKey, SshHost, SshKey};
use orm::schema::{ssh_hosts, ssh_keys};
use shared::AiAccess;
use uuid::Uuid;

use super::{internal_error, run_db, DbPool};

/// Default identities ssh tries when a host names no `IdentityFile`.
const DEFAULT_IDENTITIES: [&str; 3] = ["id_ed25519", "id_ecdsa", "id_rsa"];
/// Guards against an `Include` that includes itself.
const MAX_INCLUDE_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SshConfigHost {
    pub alias: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub identity_file: Option<String>,
    /// Alias of the jump host, when `ProxyJump` names another config host.
    pub jump_alias: Option<String>,
    /// A saved connection already has this host, port and user.
    pub exists: bool,
}

#[derive(Default)]
struct Block {
    patterns: Vec<String>,
    options: Vec<(String, String)>,
}

#[tauri::command]
#[specta::specta]
pub async fn ssh_config_scan(pool: tauri::State<'_, DbPool>) -> Result<Vec<SshConfigHost>, String> {
    let ssh_dir = ssh_dir()?;
    let pool = pool.inner().clone();
    run_db(pool, move |db| scan(db, &ssh_dir)).await
}

/// Imports the chosen aliases and returns how many connections were created.
#[tauri::command]
#[specta::specta]
pub async fn ssh_config_import(
    aliases: Vec<String>,
    pool: tauri::State<'_, DbPool>,
) -> Result<u32, String> {
    let ssh_dir = ssh_dir()?;
    let pool = pool.inner().clone();
    run_db(pool, move |db| {
        // `transaction` wants a diesel error; carry ours past it to keep the message.
        let mut failure = None;
        db.transaction(|db| {
            import(db, &ssh_dir, &aliases).map_err(|error| {
                failure = Some(error);
                diesel::result::Error::RollbackTransaction
            })
        })
        .map_err(|error| failure.take().unwrap_or_else(|| internal_error(error)))
    })
    .await
}

fn ssh_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| PathBuf::from(home).join(".ssh"))
        .map_err(|_| "HOME environment variable not set".to_string())
}

pub(super) fn scan(
    db: &mut SqliteConnection,
    ssh_dir: &Path,
) -> Result<Vec<SshConfigHost>, String> {
    let config = ssh_dir.join("config");
    let text = std::fs::read_to_string(&config)
        .map_err(|error| format!("Could not read {}: {error}", config.display()))?;
    let mut blocks = Vec::new();
    parse(&text, ssh_dir, 0, &mut blocks);
    let saved = ssh_hosts::table
        .load::<SshHost>(db)
        .map_err(internal_error)?;
    Ok(resolve(&blocks, ssh_dir)
        .into_iter()
        .map(|mut host| {
            host.exists = saved.iter().any(|saved| {
                saved.host == host.host
                    && saved.port == i32::from(host.port)
                    && saved.username == host.username
            });
            host
        })
        .collect())
}

pub(super) fn import(
    db: &mut SqliteConnection,
    ssh_dir: &Path,
    aliases: &[String],
) -> Result<u32, String> {
    let hosts = scan(db, ssh_dir)?;
    let now = Utc::now().naive_utc();
    let mut ids: HashMap<String, String> = HashMap::new();
    let mut created = Vec::new();

    for host in hosts
        .iter()
        .filter(|host| !host.exists && aliases.contains(&host.alias))
    {
        let ssh_key_id = match &host.identity_file {
            Some(path) => import_key(db, Path::new(path), now)?,
            None => None,
        };
        let id = Uuid::new_v4().to_string();
        diesel::insert_into(ssh_hosts::table)
            .values(&NewSshHost {
                id: id.clone(),
                name: host.alias.clone(),
                host: host.host.clone(),
                port: i32::from(host.port),
                username: host.username.clone(),
                ssh_key_id,
                password: None,
                group_id: None,
                created_at: now,
                updated_at: now,
                jump_host_id: None,
                ai_access: AiAccess::Ask.as_str().to_string(),
            })
            .execute(db)
            .map_err(internal_error)?;
        ids.insert(host.alias.clone(), id.clone());
        created.push((id, host));
    }

    // Link jumps once every imported host has an id. A jump to a host that was
    // not imported now falls back to a saved connection with that name.
    for (id, host) in &created {
        let Some(alias) = &host.jump_alias else {
            continue;
        };
        let jump_id = match ids.get(alias) {
            Some(jump_id) => Some(jump_id.clone()),
            None => ssh_hosts::table
                .filter(ssh_hosts::name.eq(alias))
                .select(ssh_hosts::id)
                .first::<String>(db)
                .optional()
                .map_err(internal_error)?,
        };
        let Some(jump_id) = jump_id else { continue };
        // A looping or over-deep chain from the config is left unlinked.
        if super::hosts::validate_jump_chain(db, Some(id), &jump_id).is_ok() {
            diesel::update(ssh_hosts::table.filter(ssh_hosts::id.eq(id)))
                .set(ssh_hosts::jump_host_id.eq(jump_id))
                .execute(db)
                .map_err(internal_error)?;
        }
    }

    Ok(created.len() as u32)
}

/// Stores a key file once, reusing the row when another host already uses it.
/// An unreadable file leaves the host without a key rather than failing the import.
fn import_key(
    db: &mut SqliteConnection,
    path: &Path,
    now: chrono::NaiveDateTime,
) -> Result<Option<String>, String> {
    let Ok(private_key) = std::fs::read_to_string(path) else {
        return Ok(None);
    };
    let private_key = private_key.trim().to_string();
    let existing = ssh_keys::table
        .filter(ssh_keys::private_key.eq(&private_key))
        .first::<SshKey>(db)
        .optional()
        .map_err(internal_error)?;
    if let Some(key) = existing {
        return Ok(Some(key.id));
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported key".to_string());
    diesel::insert_into(ssh_keys::table)
        .values(&NewSshKey {
            id: Uuid::new_v4().to_string(),
            name,
            kind: "inline".to_string(),
            fingerprint: None,
            private_key,
            passphrase: None,
            created_at: now,
            updated_at: now,
        })
        .get_result::<SshKey>(db)
        .map(|key| Some(key.id))
        .map_err(internal_error)
}

fn parse(text: &str, ssh_dir: &Path, depth: usize, blocks: &mut Vec<Block>) {
    // Options before the first `Host` apply to every host, like `Host *`.
    if blocks.is_empty() {
        blocks.push(Block {
            patterns: vec!["*".to_string()],
            options: Vec::new(),
        });
    }
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((keyword, value)) = line.split_once(|c: char| c.is_whitespace() || c == '=')
        else {
            continue;
        };
        let value = value.trim_start_matches(['=', ' ', '\t']).trim();
        match keyword.to_ascii_lowercase().as_str() {
            "host" => blocks.push(Block {
                patterns: value.split_whitespace().map(str::to_string).collect(),
                options: Vec::new(),
            }),
            // Nothing under a `Match` should leak into the hosts around it.
            "match" => blocks.push(Block::default()),
            "include" if depth < MAX_INCLUDE_DEPTH => {
                // Included lines continue the current block; afterwards the
                // including file resumes in that same block's scope.
                let scope = blocks
                    .last()
                    .map(|block| block.patterns.clone())
                    .unwrap_or_default();
                for path in value
                    .split_whitespace()
                    .flat_map(|pattern| include_paths(pattern, ssh_dir))
                {
                    if let Ok(text) = std::fs::read_to_string(path) {
                        parse(&text, ssh_dir, depth + 1, blocks);
                    }
                }
                blocks.push(Block {
                    patterns: scope,
                    options: Vec::new(),
                });
            }
            keyword => {
                if let Some(block) = blocks.last_mut() {
                    block
                        .options
                        .push((keyword.to_string(), unquote(value).to_string()));
                }
            }
        }
    }
}

fn include_paths(pattern: &str, ssh_dir: &Path) -> Vec<PathBuf> {
    let path = expand_home(pattern, ssh_dir);
    let path = if path.is_relative() {
        ssh_dir.join(path)
    } else {
        path
    };
    let file_pattern = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !file_pattern.contains(['*', '?']) {
        return vec![path];
    }
    let Some(dir) = path.parent() else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|entry| {
            entry
                .file_name()
                .is_some_and(|name| wildcard(&file_pattern, &name.to_string_lossy()))
        })
        .collect();
    paths.sort();
    paths
}

fn resolve(blocks: &[Block], ssh_dir: &Path) -> Vec<SshConfigHost> {
    let mut aliases: Vec<&str> = Vec::new();
    for block in blocks {
        for pattern in &block.patterns {
            if !pattern.contains(['*', '?', '!']) && !aliases.contains(&pattern.as_str()) {
                aliases.push(pattern);
            }
        }
    }
    let local_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();

    let mut hosts: Vec<SshConfigHost> = aliases
        .iter()
        .map(|alias| {
            let mut options: HashMap<&str, &str> = HashMap::new();
            for block in blocks
                .iter()
                .filter(|block| host_matches(&block.patterns, alias))
            {
                for (keyword, value) in &block.options {
                    options.entry(keyword.as_str()).or_insert(value.as_str());
                }
            }
            let identity_file = match options.get("identityfile") {
                Some(path) => Some(expand_home(path, ssh_dir)),
                None => DEFAULT_IDENTITIES
                    .iter()
                    .map(|name| ssh_dir.join(name))
                    .find(|path| path.is_file()),
            };
            SshConfigHost {
                alias: alias.to_string(),
                host: options.get("hostname").unwrap_or(alias).to_string(),
                port: options
                    .get("port")
                    .and_then(|port| port.parse().ok())
                    .unwrap_or(22),
                username: options
                    .get("user")
                    .map(|user| user.to_string())
                    .unwrap_or_else(|| local_user.clone()),
                identity_file: identity_file.map(|path| path.to_string_lossy().into_owned()),
                jump_alias: options
                    .get("proxyjump")
                    .filter(|value| !value.eq_ignore_ascii_case("none"))
                    // ponytail: only the last hop of `a,b` is linked; earlier hops
                    // come from the jump host's own ProxyJump, which covers the usual setup.
                    .and_then(|value| value.rsplit(',').next())
                    .map(|hop| hop.trim().to_string()),
                exists: false,
            }
        })
        .collect();

    // Only a ProxyJump that names another config host can become a link.
    let known: Vec<String> = hosts.iter().map(|host| host.alias.clone()).collect();
    for host in &mut hosts {
        if host
            .jump_alias
            .as_ref()
            .is_some_and(|alias| !known.contains(alias))
        {
            host.jump_alias = None;
        }
    }
    hosts
}

fn host_matches(patterns: &[String], alias: &str) -> bool {
    let mut matched = false;
    for pattern in patterns {
        match pattern.strip_prefix('!') {
            Some(negated) if wildcard(negated, alias) => return false,
            Some(_) => {}
            None => matched |= wildcard(pattern, alias),
        }
    }
    matched
}

fn wildcard(pattern: &str, text: &str) -> bool {
    let (pattern, text): (Vec<char>, Vec<char>) =
        (pattern.chars().collect(), text.chars().collect());
    let (mut p, mut t, mut star, mut mark) = (0, 0, None, 0);
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            mark = t;
            p += 1;
        } else if let Some(star) = star {
            p = star + 1;
            mark += 1;
            t = mark;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|c| *c == '*')
}

fn expand_home(path: &str, ssh_dir: &Path) -> PathBuf {
    let home = ssh_dir.parent().unwrap_or(ssh_dir);
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(path),
    }
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hosts(text: &str, ssh_dir: &Path) -> Vec<SshConfigHost> {
        let mut blocks = Vec::new();
        parse(text, ssh_dir, 0, &mut blocks);
        resolve(&blocks, ssh_dir)
    }

    #[test]
    fn first_value_wins_and_wildcards_fill_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = "\
Host bastion
  HostName 203.0.113.5
  User ops
  IdentityFile ~/.ssh/work_key

Host db-*
  ProxyJump bastion

Host db-1 !db-2
  HostName 10.0.0.11
  Port=2222

Host *
  User fallback
  Port 22
";
        let found = hosts(config, &dir.path().join(".ssh"));
        assert_eq!(found.len(), 2, "patterns are not hosts: {found:?}");

        let bastion = &found[0];
        assert_eq!(bastion.alias, "bastion");
        assert_eq!(bastion.host, "203.0.113.5");
        assert_eq!(bastion.username, "ops");
        assert_eq!(bastion.port, 22);
        assert_eq!(
            bastion.identity_file.as_deref(),
            Some(dir.path().join(".ssh/work_key").to_str().unwrap())
        );
        assert_eq!(bastion.jump_alias, None);

        let db = &found[1];
        assert_eq!(
            (db.host.as_str(), db.port, db.username.as_str()),
            ("10.0.0.11", 2222, "fallback")
        );
        assert_eq!(db.jump_alias.as_deref(), Some("bastion"));
    }

    #[test]
    fn includes_and_default_identity_are_followed() {
        let dir = tempfile::tempdir().unwrap();
        let ssh_dir = dir.path().join(".ssh");
        std::fs::create_dir_all(ssh_dir.join("conf.d")).unwrap();
        std::fs::write(
            ssh_dir.join("conf.d/work.conf"),
            "Host work\n  HostName work.example.test\n  ProxyJump nowhere\n",
        )
        .unwrap();
        std::fs::write(ssh_dir.join("id_ed25519"), "KEY").unwrap();

        let found = hosts("Include conf.d/*.conf\nHost *\n  User me\n", &ssh_dir);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].host, "work.example.test");
        assert_eq!(found[0].username, "me");
        assert_eq!(found[0].jump_alias, None, "unknown jump hosts are dropped");
        assert_eq!(
            found[0].identity_file.as_deref(),
            Some(ssh_dir.join("id_ed25519").to_str().unwrap())
        );
    }

    #[test]
    fn negated_patterns_and_wildcards_match_like_ssh() {
        assert!(wildcard("db-*", "db-1"));
        assert!(wildcard("h?st", "host"));
        assert!(!wildcard("db-*", "web-1"));
        assert!(host_matches(&["*".into(), "!prod".into()], "dev"));
        assert!(!host_matches(&["*".into(), "!prod".into()], "prod"));
    }
}
