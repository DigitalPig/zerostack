use std::io::Write;

use crossterm::ExecutableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::terminal::Clear;

use crate::ui::utils::resolve_color;

pub enum ModelPickerAction {
    Continue,
    Selected(String),
    Cancelled,
}

pub struct ModelPicker {
    pub active: bool,
    pub query: String,
    items: Vec<String>,
    pub matches: Vec<String>,
    pub selected: usize,
    monochrome: bool,
}

impl ModelPicker {
    pub fn new() -> Self {
        ModelPicker {
            active: false,
            query: String::new(),
            items: Vec::new(),
            matches: Vec::new(),
            selected: 0,
            monochrome: false,
        }
    }

    pub fn set_monochrome(&mut self, monochrome: bool) {
        self.monochrome = monochrome;
    }

    fn color(&self, color: Color) -> Color {
        resolve_color(color, self.monochrome)
    }

    pub fn activate(&mut self, items: Vec<String>, current: &str) {
        self.active = true;
        self.query.clear();
        self.items = items;
        self.filter();
        self.selected = self
            .matches
            .iter()
            .position(|m| m == current)
            .unwrap_or(0);
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn char_input(&mut self, c: char) {
        self.query.push(c);
        self.filter();
    }

    pub fn backspace(&mut self) -> bool {
        if self.query.is_empty() {
            return false;
        }
        self.query.pop();
        self.filter();
        true
    }

    fn filter(&mut self) {
        let query_lower = self.query.to_lowercase();
        self.matches = self
            .items
            .iter()
            .filter(|m| m.to_lowercase().contains(&query_lower))
            .take(50)
            .cloned()
            .collect();
        self.selected = 0;
    }

    pub fn select_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + 1) % self.matches.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.matches.is_empty() {
            self.selected = if self.selected == 0 {
                self.matches.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn selected_model(&self) -> Option<&str> {
        self.matches.get(self.selected).map(|s| s.as_str())
    }

    pub fn draw(&self) -> std::io::Result<()> {
        if !self.active {
            return Ok(());
        }
        let (cols, rows) = crossterm::terminal::size()?;
        let mut stdout = std::io::stdout();

        // Reserve one row for the search bar above the list
        let max_items = (rows.saturating_sub(5)).min(12) as usize;

        // Search bar row sits just above the input line (row rows-3)
        let search_row = rows.saturating_sub(3);
        stdout.execute(MoveTo(0, search_row))?;
        write!(
            stdout,
            "{}",
            Clear(crossterm::terminal::ClearType::CurrentLine)
        )?;
        write!(stdout, "{}", SetForegroundColor(self.color(Color::DarkGrey)))?;
        let query_display = if self.query.is_empty() {
            "type to filter".to_string()
        } else {
            format!("{}_", self.query)
        };
        let search_label = format!("  / {}", query_display);
        let search_truncated: String = search_label
            .chars()
            .take(cols as usize)
            .collect();
        write!(stdout, "{}", search_truncated)?;
        write!(stdout, "{}", ResetColor)?;

        if self.matches.is_empty() {
            let r = rows.saturating_sub(4);
            stdout.execute(MoveTo(0, r))?;
            write!(stdout, "{}", Clear(crossterm::terminal::ClearType::CurrentLine))?;
            write!(stdout, "{}", SetForegroundColor(self.color(Color::DarkGrey)))?;
            write!(stdout, "  no matches")?;
            write!(stdout, "{}", ResetColor)?;
            stdout.flush()?;
            return Ok(());
        }

        let list_height = max_items.min(self.matches.len());
        let start_idx = self
            .selected
            .saturating_sub(list_height / 2)
            .min(self.matches.len().saturating_sub(list_height));
        let end_idx = (start_idx + list_height).min(self.matches.len());

        let top_row = rows.saturating_sub(4).saturating_sub(list_height as u16);

        for i in start_idx..end_idx {
            let render_row = top_row + (i - start_idx) as u16;
            stdout.execute(MoveTo(0, render_row))?;
            write!(
                stdout,
                "{}",
                Clear(crossterm::terminal::ClearType::CurrentLine)
            )?;

            let name = &self.matches[i];
            let truncated: String = name
                .chars()
                .take(cols.saturating_sub(3) as usize)
                .collect();

            if i == self.selected {
                write!(stdout, "{}", SetForegroundColor(self.color(Color::Green)))?;
                write!(stdout, "▸ {}", truncated)?;
            } else {
                write!(
                    stdout,
                    "{}",
                    SetForegroundColor(self.color(Color::DarkGrey))
                )?;
                write!(stdout, "  {}", truncated)?;
            }
            write!(stdout, "{}", ResetColor)?;
        }
        stdout.flush()?;
        Ok(())
    }
}
