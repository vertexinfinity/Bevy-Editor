//! Plain multi-line text buffer with a caret, used by the code editor widget.
//!
//! Columns count `char`s (not bytes) so non-ASCII text cannot split a code point.

/// Spaces inserted by Tab.
pub const TAB: &str = "    ";

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct Snapshot {
    lines: Vec<String>,
    line: usize,
    col: usize,
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    pub lines: Vec<String>,
    pub line: usize,
    pub col: usize,
    /// Column the caret tries to keep while moving vertically.
    goal_col: usize,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Groups consecutive typing into one undo step.
    typing: bool,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new("")
    }
}

fn char_len(s: &str) -> usize {
    s.chars().count()
}

fn byte_index(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len())
}

impl TextBuffer {
    pub fn new(text: &str) -> Self {
        let text = text.replace("\r\n", "\n").replace('\t', TAB);
        let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Self { lines, line: 0, col: 0, goal_col: 0, undo: Vec::new(), redo: Vec::new(), typing: false }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.lines.get(line).map(|l| char_len(l)).unwrap_or(0)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { lines: self.lines.clone(), line: self.line, col: self.col }
    }

    /// Saves an undo point before an edit; `typing` edits merge with the previous one.
    fn checkpoint(&mut self, typing: bool) {
        if !(typing && self.typing) {
            self.undo.push(self.snapshot());
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
        }
        self.typing = typing;
        self.redo.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.undo.pop() else { return false };
        self.redo.push(self.snapshot());
        self.restore(snapshot);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.redo.pop() else { return false };
        self.undo.push(self.snapshot());
        self.restore(snapshot);
        true
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.lines = snapshot.lines;
        self.line = snapshot.line.min(self.lines.len() - 1);
        self.col = snapshot.col.min(self.line_len(self.line));
        self.goal_col = self.col;
        self.typing = false;
    }

    fn clamp(&mut self) {
        self.line = self.line.min(self.lines.len().saturating_sub(1));
        self.col = self.col.min(self.line_len(self.line));
    }

    pub fn set_caret(&mut self, line: usize, col: usize) {
        self.line = line;
        self.col = col;
        self.clamp();
        self.goal_col = self.col;
        self.typing = false;
    }

    pub fn insert_str(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n").replace('\t', TAB);
        if text.is_empty() {
            return;
        }
        self.checkpoint(!text.contains('\n') && text.chars().all(|c| !c.is_whitespace() || c == ' '));
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.split_line();
            }
            let line = &mut self.lines[self.line];
            let at = byte_index(line, self.col);
            line.insert_str(at, part);
            self.col += char_len(part);
        }
        self.goal_col = self.col;
    }

    fn split_line(&mut self) {
        let line = &mut self.lines[self.line];
        let at = byte_index(line, self.col);
        let rest = line.split_off(at);
        self.lines.insert(self.line + 1, rest);
        self.line += 1;
        self.col = 0;
    }

    /// Enter: splits the line and keeps the current indentation (+1 level after `{`).
    pub fn newline(&mut self) {
        self.checkpoint(false);
        let current = &self.lines[self.line];
        let indent: String = current.chars().take_while(|c| *c == ' ').collect();
        let before: String = current.chars().take(self.col).collect();
        let after: String = current.chars().skip(self.col).collect();
        let opens = before.trim_end().ends_with('{');
        let closes = after.trim_start().starts_with('}');
        self.split_line();
        let mut extra = indent.clone();
        if opens {
            extra.push_str(TAB);
        }
        // Drop leading spaces carried over from the split.
        let carried = self.lines[self.line].trim_start().to_string();
        self.lines[self.line] = format!("{extra}{carried}");
        self.col = char_len(&extra);
        if opens && closes {
            let line = self.lines[self.line].clone();
            let at = byte_index(&line, self.col);
            self.lines[self.line] = line[..at].to_string();
            self.lines.insert(self.line + 1, format!("{indent}{}", &line[at..]));
        }
        self.goal_col = self.col;
    }

    pub fn backspace(&mut self) {
        if self.col == 0 && self.line == 0 {
            return;
        }
        self.checkpoint(false);
        if self.col > 0 {
            let line = &self.lines[self.line];
            // Delete a whole indent level inside leading whitespace.
            let leading = line.chars().take(self.col).all(|c| c == ' ');
            let remove = if leading && self.col >= TAB.len() { (self.col - 1) % TAB.len() + 1 } else { 1 };
            let start = byte_index(line, self.col - remove);
            let end = byte_index(line, self.col);
            self.lines[self.line].replace_range(start..end, "");
            self.col -= remove;
        } else {
            let line = self.lines.remove(self.line);
            self.line -= 1;
            self.col = self.line_len(self.line);
            self.lines[self.line].push_str(&line);
        }
        self.goal_col = self.col;
    }

    pub fn delete(&mut self) {
        if self.col >= self.line_len(self.line) {
            if self.line + 1 < self.lines.len() {
                self.checkpoint(false);
                let next = self.lines.remove(self.line + 1);
                self.lines[self.line].push_str(&next);
            }
            return;
        }
        self.checkpoint(false);
        let line = &self.lines[self.line];
        let start = byte_index(line, self.col);
        let end = byte_index(line, self.col + 1);
        self.lines[self.line].replace_range(start..end, "");
    }

    /// Tab inserts spaces to the next indent stop.
    pub fn tab(&mut self) {
        let pad = TAB.len() - self.col % TAB.len();
        self.checkpoint(false);
        let line = &mut self.lines[self.line];
        let at = byte_index(line, self.col);
        line.insert_str(at, &" ".repeat(pad));
        self.col += pad;
        self.goal_col = self.col;
    }

    /// Shift+Tab removes one indent level from the current line.
    pub fn outdent(&mut self) {
        let spaces = self.lines[self.line].chars().take_while(|c| *c == ' ').count().min(TAB.len());
        if spaces == 0 {
            return;
        }
        self.checkpoint(false);
        self.lines[self.line].replace_range(0..spaces, "");
        self.col = self.col.saturating_sub(spaces);
        self.goal_col = self.col;
    }

    pub fn duplicate_line(&mut self) {
        self.checkpoint(false);
        let copy = self.lines[self.line].clone();
        self.lines.insert(self.line + 1, copy);
        self.line += 1;
        self.clamp();
    }

    pub fn delete_line(&mut self) {
        self.checkpoint(false);
        if self.lines.len() == 1 {
            self.lines[0].clear();
        } else {
            self.lines.remove(self.line);
        }
        self.clamp();
        self.goal_col = self.col;
    }

    pub fn left(&mut self) {
        self.typing = false;
        if self.col > 0 {
            self.col -= 1;
        } else if self.line > 0 {
            self.line -= 1;
            self.col = self.line_len(self.line);
        }
        self.goal_col = self.col;
    }

    pub fn right(&mut self) {
        self.typing = false;
        if self.col < self.line_len(self.line) {
            self.col += 1;
        } else if self.line + 1 < self.lines.len() {
            self.line += 1;
            self.col = 0;
        }
        self.goal_col = self.col;
    }

    /// Ctrl+Left / Ctrl+Right: jump over a word.
    pub fn word_left(&mut self) {
        if self.col == 0 {
            self.left();
            return;
        }
        let chars: Vec<char> = self.lines[self.line].chars().collect();
        let mut col = self.col;
        while col > 0 && !chars[col - 1].is_alphanumeric() && chars[col - 1] != '_' {
            col -= 1;
        }
        while col > 0 && (chars[col - 1].is_alphanumeric() || chars[col - 1] == '_') {
            col -= 1;
        }
        self.set_caret(self.line, col);
    }

    pub fn word_right(&mut self) {
        let chars: Vec<char> = self.lines[self.line].chars().collect();
        if self.col >= chars.len() {
            self.right();
            return;
        }
        let mut col = self.col;
        while col < chars.len() && !chars[col].is_alphanumeric() && chars[col] != '_' {
            col += 1;
        }
        while col < chars.len() && (chars[col].is_alphanumeric() || chars[col] == '_') {
            col += 1;
        }
        self.set_caret(self.line, col);
    }

    pub fn vertical(&mut self, delta: isize) {
        self.typing = false;
        let target = (self.line as isize + delta).clamp(0, self.lines.len() as isize - 1) as usize;
        self.line = target;
        self.col = self.goal_col.min(self.line_len(self.line));
    }

    /// Home toggles between the first non-space character and column 0.
    pub fn home(&mut self) {
        let indent = self.lines[self.line].chars().take_while(|c| *c == ' ').count();
        let col = if self.col == indent { 0 } else { indent };
        self.set_caret(self.line, col);
    }

    pub fn end(&mut self) {
        let len = self.line_len(self.line);
        self.set_caret(self.line, len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_enter_backspace_and_undo() {
        let mut b = TextBuffer::new("fn a() {}");
        b.set_caret(0, 8);
        b.newline();
        assert_eq!(b.lines, vec!["fn a() {", "    ", "}"]);
        assert_eq!((b.line, b.col), (1, 4));
        b.insert_str("x");
        b.insert_str("y");
        assert_eq!(b.lines[1], "    xy");
        b.backspace();
        assert_eq!(b.lines[1], "    x");
        b.undo();
        assert_eq!(b.lines[1], "    xy");
        b.undo();
        assert_eq!(b.lines[1], "    ");
        b.redo();
        assert_eq!(b.lines[1], "    xy");
    }

    #[test]
    fn navigation_and_line_joins() {
        let mut b = TextBuffer::new("ab\n    cd\nlonger line");
        b.set_caret(2, 9);
        b.vertical(-1);
        assert_eq!((b.line, b.col), (1, 6));
        b.home();
        assert_eq!(b.col, 4);
        b.home();
        assert_eq!(b.col, 0);
        b.backspace();
        assert_eq!(b.lines[0], "ab    cd");
        b.end();
        b.delete();
        assert_eq!(b.lines.len(), 1);
        assert_eq!(b.text(), "ab    cdlonger line");
        b.set_caret(0, 0);
        b.tab();
        assert_eq!(b.col, 4);
        b.outdent();
        assert_eq!(b.col, 0);
        b.word_right();
        assert_eq!(b.col, 2);
    }

    #[test]
    fn unicode_columns_are_chars() {
        let mut b = TextBuffer::new("é");
        b.end();
        b.insert_str("ü");
        assert_eq!(b.text(), "éü");
        b.backspace();
        assert_eq!(b.text(), "é");
    }
}
