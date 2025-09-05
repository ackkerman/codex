use std::path::Path;
use std::process::Command;

use walkdir::WalkDir;

use crate::ContentItem;
use crate::ResponseItem;

pub(crate) fn gather_csv_context(cwd: &Path) -> Option<ResponseItem> {
    let mut sections = Vec::new();
    for entry in WalkDir::new(cwd).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_none_or(|e| e.to_lowercase() != "csv")
        {
            continue;
        }
        let query = format!("select * from {} limit 5", path.to_string_lossy());
        let output = Command::new("q")
            .args(["-d", ",", "-H", &query])
            .output()
            .ok()?;
        if !output.status.success() {
            continue;
        }
        let data = String::from_utf8_lossy(&output.stdout);
        sections.push(format!("{}:\n{}", path.display(), data));
    }
    if sections.is_empty() {
        return None;
    }
    Some(ResponseItem::Message {
        id: None,
        role: "user".to_string(),
        content: vec![ContentItem::InputText {
            text: sections.join("\n"),
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extracts_csv_context() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("data.csv");
        fs::write(&csv_path, "id,name\n1,Alice\n2,Bob\n").unwrap();

        let bin_dir = dir.path().join("bin");
        std::fs::create_dir(&bin_dir).unwrap();
        let fake_q = bin_dir.join("q");
        fs::write(
            &fake_q,
            "#!/bin/sh\necho 'id,name'\necho '1,Alice'\necho '2,Bob'\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&fake_q).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake_q, perms).unwrap();
        unsafe {
            std::env::set_var(
                "PATH",
                format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap()),
            );
        }

        let item = gather_csv_context(dir.path()).unwrap();
        if let ResponseItem::Message { content, .. } = item {
            if let ContentItem::InputText { text } = &content[0] {
                assert!(text.contains("Alice"));
                assert!(text.contains("Bob"));
            } else {
                panic!("unexpected content item");
            }
        } else {
            panic!("unexpected response item");
        }
    }
}
