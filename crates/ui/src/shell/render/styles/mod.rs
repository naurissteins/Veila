mod color;
mod surface;
mod text;

use veila_renderer::ClearColor;

use super::super::{ShellState, ShellStatus};

pub(crate) use color::percent_to_alpha;

impl ShellState {
    fn accent_color(&self) -> ClearColor {
        match &self.status {
            ShellStatus::Idle => self.theme.input_border.with_alpha(210),
            ShellStatus::Pending { .. } => self.theme.pending,
            ShellStatus::Rejected { .. } => self.theme.rejected,
        }
    }
}
