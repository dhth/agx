use console::{Color, style};
use similar::ChangeTag;
use similar::TextDiff;
use std::cmp::max;

#[derive(Clone, Debug)]
pub struct Diff {
    pub hunks: Vec<DiffHunk>,
}

#[derive(Clone, Debug)]
pub struct DiffHunk {
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug)]
pub struct DiffLine {
    pub kind: DiffOperation,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub inline_changes: Vec<InlineChange>,
}

#[derive(Clone, Debug)]
pub struct InlineChange {
    pub value: String,
    pub emphasized: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DiffOperation {
    Insert,
    Delete,
    Equal,
}

impl DiffOperation {
    pub fn sign(&self) -> &'static str {
        match self {
            DiffOperation::Delete => "-",
            DiffOperation::Insert => "+",
            DiffOperation::Equal => " ",
        }
    }
}

impl From<ChangeTag> for DiffOperation {
    fn from(value: ChangeTag) -> Self {
        match value {
            ChangeTag::Delete => DiffOperation::Delete,
            ChangeTag::Insert => DiffOperation::Insert,
            ChangeTag::Equal => DiffOperation::Equal,
        }
    }
}

impl Diff {
    pub fn new(old: &str, new: &str) -> Option<Self> {
        let diff = TextDiff::from_lines(old, new);

        if diff.ops().is_empty() {
            return None;
        }

        let mut hunks = Vec::new();
        for group in diff.grouped_ops(3) {
            let mut lines = Vec::new();

            for op in group {
                for change in diff.iter_inline_changes(&op) {
                    let operation = DiffOperation::from(change.tag());
                    let mut inline_changes = Vec::new();

                    for (emphasized, value) in change.iter_strings_lossy() {
                        inline_changes.push(InlineChange {
                            value: value.to_string(),
                            emphasized,
                        });
                    }

                    lines.push(DiffLine {
                        kind: operation,
                        old_line_num: change.old_index(),
                        new_line_num: change.new_index(),
                        inline_changes,
                    });
                }
            }

            hunks.push(DiffHunk { lines });
        }

        if hunks.is_empty() {
            return None;
        }

        Some(Diff { hunks })
    }

    pub fn line_num_padding(&self) -> usize {
        let largest_line_num = self
            .hunks
            .iter()
            .flat_map(|hunk| hunk.lines.iter())
            .map(|diff_line| {
                max(
                    diff_line.old_line_num.unwrap_or_default(),
                    diff_line.new_line_num.unwrap_or_default(),
                )
            })
            .max()
            .unwrap_or_default();

        max(num_digits(largest_line_num) + 2, 4)
    }

    pub fn get_terminal_output(&self) -> String {
        self.get_output(true)
    }

    fn get_output(&self, color: bool) -> String {
        if self.hunks.is_empty() {
            return String::new();
        }

        let line_number_padding = self.line_num_padding();
        let mut lines = Vec::new();

        for (idx, hunk) in self.hunks.iter().enumerate() {
            if idx > 0 {
                lines.push(format!("{:-^80}", "-"));
            }

            for diff_line in &hunk.lines {
                let sign = diff_line.kind.sign();
                let old_line = diff_line
                    .old_line_num
                    .map(|n| format!("{:<padding$}", n + 1, padding = line_number_padding))
                    .unwrap_or_else(|| " ".repeat(line_number_padding));

                let new_line = diff_line
                    .new_line_num
                    .map(|n| format!("{:<padding$}", n + 1, padding = line_number_padding))
                    .unwrap_or_else(|| " ".repeat(line_number_padding));

                if color {
                    let (line_color, sign_str) = match diff_line.kind {
                        DiffOperation::Delete => (Some(Color::Red), sign),
                        DiffOperation::Insert => (Some(Color::Green), sign),
                        DiffOperation::Equal => (None, sign),
                    };

                    let old_line_styled = style(old_line.clone()).dim().to_string();
                    let new_line_styled = style(new_line.clone()).dim().to_string();
                    let sign_styled = if let Some(c) = line_color {
                        style(&sign_str).fg(c).bold().to_string()
                    } else {
                        sign_str.to_string()
                    };

                    let mut line_content =
                        format!("{}{}|{}", old_line_styled, new_line_styled, sign_styled);

                    for inline_change in &diff_line.inline_changes {
                        let value = inline_change.value.trim_end_matches('\n');
                        let formatted_value = if inline_change.emphasized {
                            if let Some(c) = line_color {
                                style(value).fg(c).underlined().on_black().to_string()
                            } else {
                                style(value).underlined().on_black().to_string()
                            }
                        } else if let Some(c) = line_color {
                            style(value).fg(c).to_string()
                        } else {
                            value.to_string()
                        };
                        line_content.push_str(&formatted_value);
                    }

                    lines.push(line_content);
                } else {
                    let mut line_spans = vec![old_line, new_line, format!("|{sign}")];

                    for inline_change in &diff_line.inline_changes {
                        let value = inline_change.value.trim_end_matches('\n');
                        if inline_change.emphasized {
                            line_spans.push(format!("⸢{}⸣", value));
                        } else {
                            line_spans.push(value.to_string());
                        }
                    }

                    lines.push(line_spans.join(""));
                }
            }
        }

        lines.join("\n")
    }
}

fn num_digits(n: usize) -> usize {
    n.checked_ilog10().map_or(1, |d| d + 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;

    #[test]
    fn creating_simple_diff_works() {
        // GIVEN
        let diff = Diff::new(
            "
line 1
line 2
line 3
",
            "
line 1 (changed)
new line
line 2
(prefix) line 3 ( changed)
",
        )
        .expect("diff should've been created");

        // WHEN
        // THEN
        assert_snapshot!(diff.get_output(false), @r"
        1   1   | 
        2       |-line 1
            2   |+line 1⸢ (changed)⸣
            3   |+⸢new line⸣
        3   4   | line 2
        4       |-line 3
            5   |+⸢(prefix) ⸣line 3⸢ ( changed)⸣
        ");
    }

    #[test]
    fn creating_diff_with_multiple_hunks_works() {
        // GIVEN
        let diff = Diff::new(
            "
line 1
line 2
line 3
line 4
line 5
line 6
line 7
line 8
line 9
",
            "
line 1 (changed)
line 2
line 3
line 4
line 5
line 6
line 7
line 8
(prefix) line 9 (changed)
",
        )
        .expect("diff should've been created");

        // WHEN
        // THEN
        assert_snapshot!(diff.get_output(false), @r"
        1   1   | 
        2       |-line 1
            2   |+line 1⸢ (changed)⸣
        3   3   | line 2
        4   4   | line 3
        5   5   | line 4
        --------------------------------------------------------------------------------
        7   7   | line 6
        8   8   | line 7
        9   9   | line 8
        10      |-line 9
            10  |+⸢(prefix) ⸣line 9⸢ (changed)⸣
        ");
    }

    #[test]
    fn diff_adjusts_padding_for_line_numbers_accordinly() {
        // GIVEN
        let mut lines = (1..=10001).map(|n| format!("line {n}")).collect::<Vec<_>>();
        let old = lines.join("\n");

        lines[8] = "line 9 (modified)".to_string();
        lines[9] = "line 10 (modified)".to_string();

        lines[998] = "line 999 (modified)".to_string();
        lines[999] = "line 1000 (modified)".to_string();

        lines[9998] = "line 9999 (modified)".to_string();
        lines[9999] = "line 10000 (modified)".to_string();

        let new = lines.join("\n");
        let diff = Diff::new(&old, &new).expect("diff should've been created");

        // WHEN
        // THEN
        assert_snapshot!(diff.get_output(false), @r"
        6      6      | line 6
        7      7      | line 7
        8      8      | line 8
        9             |-line 9
        10            |-line 10
               9      |+line 9⸢ (modified)⸣
               10     |+line 10⸢ (modified)⸣
        11     11     | line 11
        12     12     | line 12
        13     13     | line 13
        --------------------------------------------------------------------------------
        996    996    | line 996
        997    997    | line 997
        998    998    | line 998
        999           |-line 999
        1000          |-line 1000
               999    |+line 999⸢ (modified)⸣
               1000   |+line 1000⸢ (modified)⸣
        1001   1001   | line 1001
        1002   1002   | line 1002
        1003   1003   | line 1003
        --------------------------------------------------------------------------------
        9996   9996   | line 9996
        9997   9997   | line 9997
        9998   9998   | line 9998
        9999          |-line 9999
        10000         |-line 10000
               9999   |+line 9999⸢ (modified)⸣
               10000  |+line 10000⸢ (modified)⸣
        10001  10001  | line 10001
        ");
    }

    #[test]
    fn creating_a_diff_with_no_changes_works() {
        // GIVEN
        let diff = Diff::new("text", "text");

        // WHEN
        // THEN
        assert!(diff.is_none());
    }
}
