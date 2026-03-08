use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{KbError, Result};
use crate::types::KbConfig;

const KB_DIR: &str = ".kb";
const CONFIG_FILE: &str = "kb.config.yaml";
const EXPERTISE_DIR: &str = "expertise";

pub const GITATTRIBUTES_LINE: &str = ".kb/expertise/*.jsonl merge=union";

pub fn get_kb_dir(cwd: &Path) -> PathBuf {
    cwd.join(KB_DIR)
}

pub fn get_config_path(cwd: &Path) -> PathBuf {
    get_kb_dir(cwd).join(CONFIG_FILE)
}

pub fn get_expertise_dir(cwd: &Path) -> PathBuf {
    get_kb_dir(cwd).join(EXPERTISE_DIR)
}

pub fn get_expertise_path(domain: &str, cwd: &Path) -> Result<PathBuf> {
    validate_domain_name(domain)?;
    Ok(get_expertise_dir(cwd).join(format!("{domain}.jsonl")))
}

pub fn validate_domain_name(domain: &str) -> Result<()> {
    let re = regex::Regex::new(r"^[a-zA-Z0-9][a-zA-Z0-9_-]*$").unwrap();
    if !re.is_match(domain) {
        return Err(KbError::InvalidDomainName(domain.to_string()));
    }
    Ok(())
}

pub fn read_config(cwd: &Path) -> Result<KbConfig> {
    let config_path = get_config_path(cwd);
    let content = fs::read_to_string(&config_path)?;
    let config: KbConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

pub fn write_config(config: &KbConfig, cwd: &Path) -> Result<()> {
    let config_path = get_config_path(cwd);
    let content = serde_yaml::to_string(config)?;
    fs::write(&config_path, content)?;
    Ok(())
}

pub fn ensure_kb_dir(cwd: &Path) -> Result<()> {
    if !get_kb_dir(cwd).is_dir() {
        return Err(KbError::NotInitialized);
    }
    Ok(())
}

pub fn ensure_domain_exists(config: &KbConfig, domain: &str) -> Result<()> {
    if !config.domains.contains(&domain.to_string()) {
        let available = if config.domains.is_empty() {
            "(none)".to_string()
        } else {
            config.domains.join(", ")
        };
        return Err(KbError::DomainNotFound {
            domain: domain.to_string(),
            available,
        });
    }
    Ok(())
}

pub fn init_kb_dir(cwd: &Path) -> Result<()> {
    let kb_dir = get_kb_dir(cwd);
    let expertise_dir = get_expertise_dir(cwd);
    let sessions_dir = kb_dir.join("sessions");
    fs::create_dir_all(&kb_dir)?;
    fs::create_dir_all(&expertise_dir)?;
    fs::create_dir_all(&sessions_dir)?;

    // Only write default config if none exists
    let config_path = get_config_path(cwd);
    if !config_path.exists() {
        write_config(&KbConfig::default(), cwd)?;
    }

    // Create or append .gitattributes
    let gitattributes_path = cwd.join(".gitattributes");
    let existing = fs::read_to_string(&gitattributes_path).unwrap_or_default();
    if !existing.contains(GITATTRIBUTES_LINE) {
        let separator = if !existing.is_empty() && !existing.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        fs::write(
            &gitattributes_path,
            format!("{existing}{separator}{GITATTRIBUTES_LINE}\n"),
        )?;
    }

    // Add session/log files to .gitignore (local-only data)
    let gitignore_path = cwd.join(".gitignore");
    let gitignore_existing = fs::read_to_string(&gitignore_path).unwrap_or_default();
    let ignore_lines = [
        ".kb/sessions/",
        ".kb/access.jsonl",
        ".kb/changelog.jsonl",
    ];
    let mut additions = String::new();
    for line in &ignore_lines {
        if !gitignore_existing.contains(line) {
            additions.push_str(line);
            additions.push('\n');
        }
    }
    if !additions.is_empty() {
        let separator = if !gitignore_existing.is_empty() && !gitignore_existing.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        fs::write(
            &gitignore_path,
            format!("{gitignore_existing}{separator}{additions}"),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_domain_names() {
        assert!(validate_domain_name("rust").is_ok());
        assert!(validate_domain_name("my-domain").is_ok());
        assert!(validate_domain_name("domain_2").is_ok());
        assert!(validate_domain_name("A123").is_ok());
    }

    #[test]
    fn invalid_domain_names() {
        assert!(validate_domain_name("").is_err());
        assert!(validate_domain_name("-starts-with-dash").is_err());
        assert!(validate_domain_name("has spaces").is_err());
        assert!(validate_domain_name("has.dots").is_err());
    }

    #[test]
    fn init_creates_structure() {
        let tmp = tempfile::tempdir().unwrap();
        init_kb_dir(tmp.path()).unwrap();

        assert!(get_kb_dir(tmp.path()).is_dir());
        assert!(get_config_path(tmp.path()).exists());
        assert!(get_expertise_dir(tmp.path()).is_dir());
        assert!(tmp.path().join(".gitattributes").exists());

        let config = read_config(tmp.path()).unwrap();
        assert_eq!(config.version, "1");
    }

    #[test]
    fn init_preserves_existing_config() {
        let tmp = tempfile::tempdir().unwrap();
        init_kb_dir(tmp.path()).unwrap();

        // Modify config
        let mut config = read_config(tmp.path()).unwrap();
        config.domains.push("test".to_string());
        write_config(&config, tmp.path()).unwrap();

        // Re-init should not overwrite
        init_kb_dir(tmp.path()).unwrap();
        let config = read_config(tmp.path()).unwrap();
        assert_eq!(config.domains, vec!["test"]);
    }
}
