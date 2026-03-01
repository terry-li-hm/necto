use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

const SKIP: &[&str] = &["TEMPLATE.md", ".git", ".archive", ".cache", ".gitignore", "config.json"];

fn home() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).expect("$HOME not set")
}

fn targets(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".claude/skills"),
        home.join(".opencode/skills"),
        home.join(".codex/skills"),
        home.join(".agents/skills"),
    ]
}

fn sync_skills(home: &Path, dry_run: bool) -> usize {
    let skills_dir = home.join("skills");
    let targets = targets(home);

    if !dry_run {
        for t in &targets {
            fs::create_dir_all(t).ok();
        }
        for t in &targets {
            if let Ok(entries) = fs::read_dir(t) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_symlink() && !path.exists() {
                        fs::remove_file(&path).ok();
                    }
                }
            }
        }
    }

    let mut count = 0;
    let mut entries: Vec<_> = fs::read_dir(&skills_dir)
        .expect("cannot read ~/skills/")
        .flatten()
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let item = entry.path();
        let name = item.file_name().unwrap().to_string_lossy();
        if SKIP.iter().any(|s| *s == name.as_ref()) || item.is_symlink() {
            continue;
        }
        if item.is_dir() && item.join("SKILL.md").exists() {
            if dry_run {
                println!("  {}", name);
            } else {
                for t in &targets {
                    let dest = t.join(name.as_ref());
                    if dest.is_symlink() || dest.exists() {
                        fs::remove_file(&dest).ok();
                    }
                    std::os::unix::fs::symlink(&item, &dest).ok();
                }
            }
            count += 1;
        }
    }
    count
}

fn sync_mcp(home: &Path) {
    let script = home.join("scripts/mcp-sync.py");
    if !script.exists() {
        eprintln!("⚠  MCP: ~/scripts/mcp-sync.py not found, skipping");
        return;
    }
    let status = Command::new("python3").arg(&script).arg("--apply").status();
    match status {
        Ok(s) if s.success() => println!("✓  MCP: synced"),
        _ => eprintln!("⚠  MCP: sync failed"),
    }
}

fn sync_ce(home: &Path) {
    let ce_dir = home.join(".claude/plugins/marketplaces/every-marketplace");
    if !ce_dir.exists() {
        eprintln!("⚠  CE: every-marketplace not installed, skipping");
        return;
    }
    let status = Command::new("bunx")
        .args(["@every-env/compound-plugin", "install", "compound-engineering", "--to", "codex"])
        .current_dir(&ce_dir)
        .status();
    match status {
        Ok(s) if s.success() => println!("✓  CE: compound-engineering installed to Codex"),
        _ => eprintln!("⚠  CE: install failed"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: agent-sync [--full] [--check]");
        println!();
        println!("  (default)  Sync skills symlinks (fast, safe for git hooks)");
        println!("  --full     Skills + MCP + compound-engineering");
        println!("  --check    Dry run — list skills, no changes");
        exit(0);
    }

    let full = args.iter().any(|a| a == "--full");
    let check = args.iter().any(|a| a == "--check");
    let home = home();

    if check {
        println!("Skills in ~/skills/:");
        let count = sync_skills(&home, true);
        println!("{} skills", count);
        exit(0);
    }

    let count = sync_skills(&home, false);
    eprintln!("✓  Skills: {} synced", count);

    if full {
        sync_mcp(&home);
        sync_ce(&home);
        eprintln!("Done. Restart Codex/OpenCode to pick up changes.");
    }
}
