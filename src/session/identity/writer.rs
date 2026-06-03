use std::path::Path;
use std::process::Command;

use crate::model::ActorId;
use crate::session::event::{Writer, WriterRole, WriterTool};

/// Environment variable that pins the writing actor to an explicit, fully
/// qualified `actor:<scheme>:<id>` identity, taking precedence over the local
/// Git identity. Intended for callers that drive `shore` on behalf of a known
/// actor — for example a federation bridge forwarding a remote reviewer's
/// decision, where the local Git identity would otherwise mis-attribute the
/// durable write to the host running the command.
pub(crate) const SHORE_ACTOR_ID_ENV: &str = "SHORE_ACTOR_ID";

pub(crate) fn writer_from_git_config(repo: &Path) -> Writer {
    Writer {
        actor_id: actor_id_for_repo(repo),
        role: WriterRole::Author,
        tool: shore_tool(),
    }
}

pub(crate) fn reviewer_from_git_config(repo: &Path) -> Writer {
    Writer {
        actor_id: actor_id_for_repo(repo),
        role: WriterRole::Reviewer,
        tool: shore_tool(),
    }
}

/// Resolve the writing actor for `repo`: an explicit `SHORE_ACTOR_ID` wins;
/// otherwise fall back to the Git identity.
fn actor_id_for_repo(repo: &Path) -> ActorId {
    resolve_actor_id(std::env::var(SHORE_ACTOR_ID_ENV).ok().as_deref(), repo)
}

/// Pure resolution seam (kept env-free for testing): use `explicit` when it is
/// a valid fully-qualified actor id, otherwise derive from Git config.
fn resolve_actor_id(explicit: Option<&str>, repo: &Path) -> ActorId {
    if let Some(value) = explicit {
        let value = value.trim();
        if is_valid_actor_id(value) {
            return ActorId::new(value.to_owned());
        }
    }
    actor_id_from_git_config(repo)
}

/// A safe, fully-qualified actor id: an `actor:` prefix, a non-empty
/// remainder, bounded length, and no whitespace or control characters. An
/// invalid value is ignored rather than trusted, so a malformed override can
/// never silently corrupt provenance.
fn is_valid_actor_id(value: &str) -> bool {
    value.len() <= 256
        && value.strip_prefix("actor:").is_some_and(|rest| {
            !rest.is_empty() && rest.chars().all(|c| !c.is_whitespace() && !c.is_control())
        })
}

fn shore_tool() -> WriterTool {
    WriterTool {
        name: "shore".to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}

fn actor_id_from_git_config(repo: &Path) -> ActorId {
    git_config_value(repo, "user.email")
        .map(|email| ActorId::new(format!("actor:git-email:{email}")))
        .or_else(|| {
            git_config_value(repo, "user.name")
                .map(|name| ActorId::new(format!("actor:git-name:{name}")))
        })
        // V1 local workflows treat missing Git identity as one local actor.
        .unwrap_or_else(|| ActorId::new("actor:local"))
}

fn git_config_value(repo: &Path, key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", key])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    #[test]
    fn writer_from_git_config_uses_author_role_and_git_identity() {
        let repo = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "author@example.com"])
            .current_dir(repo.path())
            .output()
            .unwrap();

        let writer = super::writer_from_git_config(repo.path());

        assert_eq!(
            writer.actor_id.as_str(),
            "actor:git-email:author@example.com"
        );
        assert_eq!(writer.role, crate::session::event::WriterRole::Author);
    }

    #[test]
    fn reviewer_from_git_config_uses_email_then_name_then_actor_local() {
        let email_repo = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(email_repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "reviewer@example.com"])
            .current_dir(email_repo.path())
            .output()
            .unwrap();
        let email_writer = super::reviewer_from_git_config(email_repo.path());
        assert_eq!(
            email_writer.actor_id.as_str(),
            "actor:git-email:reviewer@example.com"
        );
        assert_eq!(
            email_writer.role,
            crate::session::event::WriterRole::Reviewer
        );

        let name_repo = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(name_repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", ""])
            .current_dir(name_repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "reviewer-name"])
            .current_dir(name_repo.path())
            .output()
            .unwrap();
        let name_writer = super::reviewer_from_git_config(name_repo.path());
        assert_eq!(
            name_writer.actor_id.as_str(),
            "actor:git-name:reviewer-name"
        );
        assert_eq!(
            name_writer.role,
            crate::session::event::WriterRole::Reviewer
        );

        let local_repo = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(local_repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", ""])
            .current_dir(local_repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", ""])
            .current_dir(local_repo.path())
            .output()
            .unwrap();
        let local_writer = super::reviewer_from_git_config(local_repo.path());
        assert_eq!(local_writer.actor_id.as_str(), "actor:local");
        assert_eq!(
            local_writer.role,
            crate::session::event::WriterRole::Reviewer
        );
    }

    fn git_repo_with_email(email: &str) -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(repo.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", email])
            .current_dir(repo.path())
            .output()
            .unwrap();
        repo
    }

    #[test]
    fn explicit_actor_id_overrides_git_identity() {
        let repo = git_repo_with_email("host@example.com");
        let actor = super::resolve_actor_id(Some("actor:agent:remote-reviewer"), repo.path());
        assert_eq!(actor.as_str(), "actor:agent:remote-reviewer");
    }

    #[test]
    fn invalid_explicit_actor_id_falls_back_to_git_identity() {
        let repo = git_repo_with_email("host@example.com");
        for bad in [
            "",
            "no-prefix",
            "actor:",
            "actor:has space",
            "actor:line\nbreak",
        ] {
            let actor = super::resolve_actor_id(Some(bad), repo.path());
            assert_eq!(
                actor.as_str(),
                "actor:git-email:host@example.com",
                "invalid override {bad:?} should fall back to the Git identity"
            );
        }
    }

    #[test]
    fn missing_explicit_actor_id_uses_git_identity() {
        let repo = git_repo_with_email("host@example.com");
        let actor = super::resolve_actor_id(None, repo.path());
        assert_eq!(actor.as_str(), "actor:git-email:host@example.com");
    }
}
