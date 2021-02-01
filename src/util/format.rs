use git2::Commit;
use git_graph::print::format::format_date;
use std::fmt::Write;
use yansi::Paint;

/// Format a commit.
pub fn format(commit: &Commit, branches: String, hash_color: Option<u8>) -> Vec<String> {
    let mut out_vec = vec![];
    let mut out = String::new();

    if let Some(color) = hash_color {
        write!(out, "{}", Paint::fixed(color, &commit.id()))
    } else {
        write!(out, "{}", &commit.id())
    }
    .unwrap();

    out_vec.push(out);
    out = String::new();

    write!(out, "{}", branches).unwrap();
    out_vec.push(out);

    if commit.parent_count() > 1 {
        out = String::new();
        write!(
            out,
            "  Merge: {} {}",
            &commit.parent_id(0).unwrap().to_string()[..7],
            &commit.parent_id(1).unwrap().to_string()[..7]
        )
        .unwrap();
        out_vec.push(out);
    } else {
        out = String::new();
        out_vec.push(out);
    }

    out = String::new();
    write!(
        out,
        "Author: {} <{}>",
        commit.author().name().unwrap_or(""),
        commit.author().email().unwrap_or("")
    )
    .unwrap();
    out_vec.push(out);

    out = String::new();
    write!(
        out,
        "Date:   {}",
        format_date(commit.author().when(), "%a %b %e %H:%M:%S %Y %z")
    )
    .unwrap();
    out_vec.push(out);

    out_vec.push("".to_string());
    let mut add_line = true;
    for line in commit.message().unwrap_or("").lines() {
        if line.is_empty() {
            out_vec.push(line.to_string());
        } else {
            out_vec.push(format!("    {}", line));
        }
        add_line = !line.trim().is_empty();
    }
    if add_line {
        out_vec.push("".to_string());
    }

    out_vec
}
